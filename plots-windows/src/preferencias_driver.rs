//! Conversa com o driver sobre o `DEVMODE`: partir do padrão ou do ajuste salvo, fixar o que o
//! envio decide e perguntar à pessoa. A lógica fica atrás de `DriverDaFila`, e o arquivo de perfis é
//! entrada hostil: os bytes salvos só chegam ao driver depois de conferidos.

use crate::erro::ErroEnvio;
use crate::opcoes::{
    bandejas_de, cor_de, resolucoes_de, total_declarado, Bandeja, Cor, Escolhas, OpcoesDaFila,
    Resolucao, CARACTERES_DO_NOME_DA_BANDEJA,
};
use crate::papel::{AjustePapel, FacesDoEnvio};
use crate::preferencias::{ErroPreferencias, RespostaDaJanela};
use crate::spooler::{sistema, BufferDevmode, Impressora};
use crate::{ErroFilas, LIMITE_BUFFER_BYTES};
use windows::Win32::Foundation::{ERROR_INVALID_PRINTER_NAME, ERROR_PRINTER_NOT_FOUND};
use windows::Win32::Graphics::Gdi::{DEVMODEW, DMDUP_SIMPLEX};
use windows::Win32::Storage::Xps::{DC_BINNAMES, DC_BINS, DC_COLORDEVICE, DC_ENUMRESOLUTIONS};

/// Resposta de `DocumentPropertiesW` quando tudo correu bem ou a pessoa aceitou.
pub(crate) const IDOK: i32 = 1;
/// Resposta de `DocumentPropertiesW` quando a pessoa cancelou.
pub(crate) const IDCANCEL: i32 = 2;

/// O que o envio precisa pedir ao driver da fila. Cada chamada devolve a resposta dele, sem
/// interpretar.
pub(crate) trait DriverDaFila {
    /// Tamanho do `DEVMODE` que o driver usa.
    fn tamanho(&self) -> i32;
    /// Preenche `destino` com o padrão da fila.
    fn ler(&self, destino: &mut BufferDevmode) -> i32;
    /// Valida `entrada` e devolve em `destino` o que o driver aceitou.
    fn mesclar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode) -> i32;
    /// Abre a janela do driver a partir de `entrada`, com `dono` como janela dona.
    fn perguntar(&self, destino: &mut BufferDevmode, entrada: &BufferDevmode, dono: isize) -> i32;
}

fn ler_padrao(driver: &impl DriverDaFila) -> Result<BufferDevmode, ErroEnvio> {
    let mut padrao = BufferDevmode::com_tamanho(driver.tamanho())?;
    if driver.ler(&mut padrao) != IDOK {
        return Err(sistema("DocumentPropertiesW"));
    }
    Ok(padrao)
}

/// Confere os bytes salvos e passa pelo driver. O ajuste precisa ser do mesmo driver, na mesma
/// versão: os dados privados só fazem sentido para quem os escreveu.
fn validar_salvo(
    driver: &impl DriverDaFila,
    padrao: &BufferDevmode,
    bytes: &[u8],
) -> Result<BufferDevmode, ErroEnvio> {
    const RECUSA: ErroEnvio = ErroEnvio::Resposta("o ajuste salvo não serve para esta fila");
    let mut salvo = BufferDevmode::de_bytes(bytes).ok_or(RECUSA)?;
    if !salvo.mesma_origem_de(padrao) {
        return Err(RECUSA);
    }
    salvo.sanear_com(padrao);
    let mut validado = BufferDevmode::com_tamanho(driver.tamanho())?;
    if driver.mesclar(&mut validado, &salvo) != IDOK {
        return Err(RECUSA);
    }
    Ok(validado)
}

/// Fixa os campos do envio, faz o driver validar e confere o que voltou. `recusa` diz quem responde
/// se o driver não aceitar a fusão.
fn fixar(
    driver: &impl DriverDaFila,
    mut base: BufferDevmode,
    ajuste: AjustePapel,
    faces: FacesDoEnvio,
    recusa: impl FnOnce() -> ErroEnvio,
) -> Result<BufferDevmode, ErroEnvio> {
    base.aplicar(ajuste);
    if faces == FacesDoEnvio::Duas {
        base.pedir_duas_faces();
    }
    let mut mesclado = BufferDevmode::com_tamanho(driver.tamanho())?;
    if driver.mesclar(&mut mesclado, &base) != IDOK {
        return Err(match faces {
            FacesDoEnvio::Uma => recusa(),
            FacesDoEnvio::Duas => ErroEnvio::DuasFacesNaoAceitas,
        });
    }
    match faces {
        FacesDoEnvio::Uma => mesclado.conferir_simplex()?,
        FacesDoEnvio::Duas => mesclado.conferir_duas_faces()?,
    }
    mesclado.conferir_aplicado()?;
    Ok(mesclado)
}

/// `DEVMODE` do envio. Parte do salvo, quando há, mas o que o envio fixa vale sobre ele: o papel,
/// uma cópia, escala real, retrato, uma página por folha e as faces pedidas.
pub(crate) fn montar(
    driver: &impl DriverDaFila,
    ajuste: AjustePapel,
    faces: FacesDoEnvio,
    salvo: Option<&[u8]>,
) -> Result<BufferDevmode, ErroEnvio> {
    let padrao = ler_padrao(driver)?;
    match salvo {
        Some(bytes) => {
            let partida = validar_salvo(driver, &padrao, bytes)?;
            fixar(driver, partida, ajuste, faces, || {
                ErroEnvio::Resposta("o driver não aceitou o ajuste salvo")
            })
        }
        None => fixar(driver, padrao, ajuste, faces, || {
            sistema("DocumentPropertiesW")
        }),
    }
}

/// Só o que vem da resposta do driver pesa contra o ajuste. Uma falha do sistema, como ao criar o
/// contexto da fila, pode passar sozinha e não diz nada sobre ele.
fn culpa_do_ajuste(erro: &ErroEnvio) -> bool {
    matches!(
        erro,
        ErroEnvio::Resposta(_)
            | ErroEnvio::SimplexNaoAceito
            | ErroEnvio::DuasFacesNaoAceitas
            | ErroEnvio::PapelNaoAceito { .. }
            | ErroEnvio::ValorNaoAceito { .. }
            | ErroEnvio::ValorNaoConfirmado { .. }
    )
}

/// Roda `tentar` com o ajuste salvo e, se o driver não o aceitar, com o padrão da fila. Devolve
/// também os bytes recusados. Qualquer outra falha aparece como erro, sem envio com outro ajuste. Se
/// o padrão falhar também, o problema não era o ajuste: ele fica guardado.
pub(crate) fn com_recuo_ao_padrao<T>(
    salvo: &mut Option<Vec<u8>>,
    mut tentar: impl FnMut(Option<&[u8]>) -> Result<T, ErroEnvio>,
) -> Result<(T, Option<Vec<u8>>), ErroEnvio> {
    let Some(bytes) = salvo.as_deref() else {
        return tentar(None).map(|valor| (valor, None));
    };
    match tentar(Some(bytes)) {
        Ok(valor) => Ok((valor, None)),
        Err(erro) if culpa_do_ajuste(&erro) => {
            let valor = tentar(None)?;
            Ok((valor, salvo.take()))
        }
        Err(erro) => Err(erro),
    }
}

/// Abre a janela do driver a partir do salvo; um salvo que não serve mais abre no padrão da fila.
pub(crate) fn perguntar(
    driver: &impl DriverDaFila,
    salvo: Option<&[u8]>,
    dono: isize,
) -> Result<RespostaDaJanela, ErroEnvio> {
    let padrao = ler_padrao(driver)?;
    let partida = salvo
        .and_then(|bytes| validar_salvo(driver, &padrao, bytes).ok())
        .unwrap_or(padrao);
    let mut saida = BufferDevmode::com_tamanho(driver.tamanho())?;
    match driver.perguntar(&mut saida, &partida, dono) {
        IDOK => saida.bytes().map(RespostaDaJanela::Aceita),
        IDCANCEL => Ok(RespostaDaJanela::Cancelada),
        _ => Err(sistema("DocumentPropertiesW")),
    }
}

pub(crate) fn abrir_fila(fila: &str) -> Result<Impressora, ErroPreferencias> {
    Impressora::abrir(fila).map_err(|erro| match erro {
        ErroEnvio::Sistema { codigo, .. } if fila_inexistente(codigo) => {
            ErroPreferencias::FilaInexistente
        }
        outro => ErroPreferencias::Envio(outro),
    })
}

fn fila_inexistente(codigo: u32) -> bool {
    [ERROR_INVALID_PRINTER_NAME.0, ERROR_PRINTER_NOT_FOUND.0]
        .iter()
        .any(|falta| windows::core::HRESULT::from_win32(*falta).0 as u32 == codigo)
}

/// `DeviceCapabilities` recebe o nome da fila e a porta, não o handle aberto. A porta pode vir em
/// lista: o driver quer uma só, como em `DriverDoSistema::dc_duplex`.
fn porta_da_fila(fila: &str) -> Result<Vec<u16>, ErroPreferencias> {
    let filas = crate::listar_filas().map_err(|erro| match erro {
        ErroFilas::Sistema(codigo) => ErroPreferencias::Envio(ErroEnvio::Sistema {
            chamada: "EnumPrintersW",
            codigo,
        }),
        _ => ErroPreferencias::Envio(ErroEnvio::Resposta(
            "não foi possível ler a fila no sistema",
        )),
    })?;
    let porta = filas
        .into_iter()
        .find(|item| item.nome == fila)
        .map(|item| item.porta)
        .ok_or(ErroPreferencias::FilaInexistente)?;
    Ok(porta
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect())
}

fn ler_bandejas(impressora: &Impressora, porta: &[u16]) -> Result<Vec<Bandeja>, ErroPreferencias> {
    let total = total_declarado(impressora.capacidades(porta, DC_BINS, None))
        .map_err(ErroPreferencias::Envio)?;
    if total == 0 {
        return Ok(Vec::new());
    }
    let mut valores = vec![0i16; total];
    let devolvidos =
        total_declarado(impressora.capacidades(porta, DC_BINS, Some(valores.as_mut_ptr().cast())))
            .map_err(ErroPreferencias::Envio)?;
    valores.truncate(devolvidos.min(total));

    let total_de_nomes = total_declarado(impressora.capacidades(porta, DC_BINNAMES, None))
        .map_err(ErroPreferencias::Envio)?;
    let mut nomes = Vec::new();
    if total_de_nomes > 0 {
        nomes = vec![0u16; total_de_nomes * CARACTERES_DO_NOME_DA_BANDEJA];
        let devolvidos = total_declarado(impressora.capacidades(
            porta,
            DC_BINNAMES,
            Some(nomes.as_mut_ptr().cast()),
        ))
        .map_err(ErroPreferencias::Envio)?;
        nomes.truncate(devolvidos.min(total_de_nomes) * CARACTERES_DO_NOME_DA_BANDEJA);
    }
    Ok(bandejas_de(&valores, &nomes))
}

fn ler_resolucoes(
    impressora: &Impressora,
    porta: &[u16],
) -> Result<Vec<Resolucao>, ErroPreferencias> {
    let total = total_declarado(impressora.capacidades(porta, DC_ENUMRESOLUTIONS, None))
        .map_err(ErroPreferencias::Envio)?;
    if total == 0 {
        return Ok(Vec::new());
    }
    let mut pares = vec![0i32; total * 2];
    let devolvidos = total_declarado(impressora.capacidades(
        porta,
        DC_ENUMRESOLUTIONS,
        Some(pares.as_mut_ptr().cast()),
    ))
    .map_err(ErroPreferencias::Envio)?;
    pares.truncate(devolvidos.min(total) * 2);
    Ok(resolucoes_de(&pares))
}

fn ler_cor(impressora: &Impressora, porta: &[u16]) -> Option<Cor> {
    cor_de(impressora.capacidades(porta, DC_COLORDEVICE, None))
}

/// O ajuste salvo é a partida quando ainda serve para esta fila; se não serve, o padrão do driver.
fn partida(driver: &impl DriverDaFila, salvo: Option<&[u8]>) -> Result<BufferDevmode, ErroEnvio> {
    let padrao = ler_padrao(driver)?;
    Ok(partida_com(driver, &padrao, salvo))
}

fn partida_com(
    driver: &impl DriverDaFila,
    padrao: &BufferDevmode,
    salvo: Option<&[u8]>,
) -> BufferDevmode {
    salvo
        .and_then(|bytes| validar_salvo(driver, padrao, bytes).ok())
        .unwrap_or_else(|| padrao.clone())
}

/// O que o driver declara para a fila, com os valores que o ajuste atual usa. Só lê.
pub fn opcoes_da_fila(fila: &str, salvo: Option<&[u8]>) -> Result<OpcoesDaFila, ErroPreferencias> {
    let impressora = abrir_fila(fila)?;
    let porta = porta_da_fila(fila)?;
    let atual = partida(&impressora, salvo)
        .map_err(ErroPreferencias::Envio)?
        .escolhas_atuais();
    Ok(OpcoesDaFila {
        bandejas: ler_bandejas(&impressora, &porta)?,
        resolucoes: ler_resolucoes(&impressora, &porta)?,
        cor: ler_cor(&impressora, &porta),
        atual,
    })
}

/// Monta o `DEVMODE` a partir do ajuste salvo ou do padrão e marca só os campos escolhidos. A
/// recusa do driver sobe como erro, sem bytes para gravar.
pub(crate) fn aplicar_em(
    driver: &impl DriverDaFila,
    salvo: Option<&[u8]>,
    escolhas: &Escolhas,
) -> Result<Vec<u8>, ErroEnvio> {
    let padrao = ler_padrao(driver)?;
    let mut novo = partida_com(driver, &padrao, salvo);
    novo.repor_campos_do_padrao(&padrao, escolhas);
    novo.aplicar_escolhas(escolhas)?;
    let mut mesclado = BufferDevmode::com_tamanho(driver.tamanho())?;
    if driver.mesclar(&mut mesclado, &novo) != IDOK {
        return Err(ErroEnvio::Resposta("o driver não aceitou as opções"));
    }
    mesclado.conferir_escolhas(escolhas)?;
    mesclado.bytes()
}

/// Aplica as escolhas no ajuste salvo da fila e devolve os bytes que o chamador grava.
pub fn aplicar_opcoes_da_fila(
    fila: &str,
    salvo: Option<&[u8]>,
    escolhas: &Escolhas,
) -> Result<Vec<u8>, ErroPreferencias> {
    let impressora = abrir_fila(fila)?;
    aplicar_em(&impressora, salvo, escolhas).map_err(ErroPreferencias::Envio)
}

/// O campo público do ajuste diz duas faces. O envio continua em uma face só, e a pessoa fica sabendo.
/// O que um driver guarda nos dados privados não é legível: esse caso não aparece aqui.
pub fn ajuste_pede_duas_faces(ajuste: &[u8]) -> bool {
    BufferDevmode::de_bytes(ajuste).is_some_and(|buffer| {
        let faces = buffer.modo().dmDuplex.0;
        faces != DMDUP_SIMPLEX.0 && faces != 0
    })
}

/// Lê, sem janela, o ajuste que o driver usa como padrão da fila.
#[cfg(test)]
pub(crate) fn ajuste_padrao_da_fila(fila: &str) -> Result<Vec<u8>, ErroPreferencias> {
    let impressora = abrir_fila(fila)?;
    ler_padrao(&impressora)
        .and_then(|padrao| padrao.bytes())
        .map_err(ErroPreferencias::Envio)
}

/// Pergunta ao driver, sem janela, se ele aceita estes bytes.
#[cfg(test)]
pub(crate) fn driver_aceita_ajuste(fila: &str, ajuste: &[u8]) -> Result<(), ErroPreferencias> {
    let impressora = abrir_fila(fila)?;
    ler_padrao(&impressora)
        .and_then(|padrao| validar_salvo(&impressora, &padrao, ajuste))
        .map(|_| ())
        .map_err(ErroPreferencias::Envio)
}

impl BufferDevmode {
    /// Só aceita bytes que têm exatamente o tamanho que os próprios campos declaram.
    pub(crate) fn de_bytes(bytes: &[u8]) -> Option<Self> {
        let total = u32::try_from(bytes.len()).ok()?;
        if total > LIMITE_BUFFER_BYTES || bytes.len() < std::mem::size_of::<DEVMODEW>() {
            return None;
        }
        let campo = |deslocamento: usize| -> Option<usize> {
            let par = bytes.get(deslocamento..deslocamento.checked_add(2)?)?;
            Some(usize::from(u16::from_ne_bytes(
                <[u8; 2]>::try_from(par).ok()?,
            )))
        };
        let publico = campo(std::mem::offset_of!(DEVMODEW, dmSize))?;
        let privado = campo(std::mem::offset_of!(DEVMODEW, dmDriverExtra))?;
        // Os campos que o envio fixa ficam antes do nome do formulário.
        if publico < std::mem::offset_of!(DEVMODEW, dmFormName) {
            return None;
        }
        if publico.checked_add(privado)? != bytes.len() {
            return None;
        }
        let mut buffer = Self::com_tamanho(i32::try_from(total).ok()?).ok()?;
        buffer.preencher(bytes);
        Some(buffer)
    }

    fn modo(&self) -> &DEVMODEW {
        // Garantia: `com_tamanho` confere que o buffer tem pelo menos `size_of::<DEVMODEW>()` bytes
        // e o guarda em `u64`, que basta para o alinhamento. Todo campo é válido em qualquer padrão
        // de bits.
        unsafe { &*self.ponteiro() }
    }

    /// Mesmo tamanho e mesma versão do driver que informou `padrao`.
    fn mesma_origem_de(&self, padrao: &Self) -> bool {
        let (salvo, atual) = (self.modo(), padrao.modo());
        salvo.dmSize == atual.dmSize
            && salvo.dmDriverExtra == atual.dmDriverExtra
            && salvo.dmSpecVersion == atual.dmSpecVersion
            && salvo.dmDriverVersion == atual.dmDriverVersion
    }

    /// O arquivo é entrada hostil. Os nomes voltam aos do padrão, pois podem nem ter o zero final que
    /// o driver espera, e só valem os campos que o próprio driver informou.
    fn sanear_com(&mut self, padrao: &Self) {
        let atual = padrao.modo();
        let (nome, formulario, campos) = (atual.dmDeviceName, atual.dmFormName, atual.dmFields);
        let cabe_o_formulario = usize::from(atual.dmSize)
            >= std::mem::offset_of!(DEVMODEW, dmFormName)
                .saturating_add(std::mem::size_of_val(&formulario));
        // Garantia: mesmo tamanho e alinhamento de `modo`; a referência exclusiva vale só aqui. Com
        // um `dmSize` curto, o lugar do formulário já é dado privado do driver e fica como está.
        unsafe {
            let modo = &mut *self.ponteiro_mut();
            modo.dmDeviceName = nome;
            modo.dmFields &= campos;
            if cabe_o_formulario {
                modo.dmFormName = formulario;
            }
        }
    }

    /// Os bytes que o driver preencheu, medidos pelos campos dele e conferidos contra o buffer.
    pub(crate) fn bytes(&self) -> Result<Vec<u8>, ErroEnvio> {
        let modo = self.modo();
        let total = usize::from(modo.dmSize).saturating_add(usize::from(modo.dmDriverExtra));
        self.copia_de(total).ok_or(ErroEnvio::Resposta(
            "o driver informou um tamanho fora do buffer",
        ))
    }
}

#[cfg(test)]
#[path = "preferencias_testes.rs"]
mod testes;
