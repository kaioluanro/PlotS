//! Leitura das filas pelo spooler (`EnumPrintersW`, nível 2, e `GetDefaultPrinterW`).
//! Só lê: nenhuma fila é aberta para escrita, nenhum trabalho é criado e nenhum
//! `DEVMODE` é alterado.

use crate::estado::Estado;
use crate::texto::{deslocamento_no_buffer, texto_de_unidades, texto_utf16_le};
use crate::{e_a_impressora_padrao, ErroFilas, Fila, LIMITE_BUFFER_BYTES};
use windows::core::{Error, HRESULT, PCWSTR, PWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_INSUFFICIENT_BUFFER};
use windows::Win32::Graphics::Printing::{
    EnumPrintersW, GetDefaultPrinterW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
    PRINTER_INFO_2W,
};

/// Nível 2 devolve nome, driver, porta, atributos e estado de cada fila.
const NIVEL_INFO_2: u32 = 2;
/// A lista pode mudar entre as duas chamadas (fila adicionada ou removida); nova tentativa limitada.
const TENTATIVAS: usize = 3;

/// Lista as filas locais e as conectadas, na ordem devolvida pelo sistema.
pub(crate) fn listar() -> Result<Vec<Fila>, ErroFilas> {
    let padrao = nome_padrao()?;
    let (buffer, retornadas) = ler_tabela()?;
    decodificar(&buffer, retornadas, padrao.as_deref())
}

/// Nome da impressora padrão, ou `None` quando o sistema não tem uma.
///
/// Usa `GetDefaultPrinterW` em vez do bit de atributo de `PRINTER_INFO_2W`, que não é a
/// fonte documentada para isso.
fn nome_padrao() -> Result<Option<String>, ErroFilas> {
    let mut tamanho: u32 = 0;
    // Garantia: sem buffer (ponteiro nulo), a função só escreve no `u32` local `tamanho`,
    // que é válido e exclusivo desta chamada.
    let primeira = unsafe { GetDefaultPrinterW(None, &mut tamanho) };
    if primeira.as_bool() {
        return Err(ErroFilas::Resposta("nome padrão sem buffer"));
    }
    let erro = Error::from_thread();
    if erro.code() == HRESULT::from_win32(ERROR_FILE_NOT_FOUND.0) {
        return Ok(None);
    }
    if erro.code() != insuficiente() {
        return Err(ErroFilas::Sistema(codigo(erro.code())));
    }
    if tamanho == 0 {
        return Err(ErroFilas::Resposta("tamanho zero para o nome padrão"));
    }
    let bytes = tamanho.checked_mul(2).unwrap_or(u32::MAX);
    if bytes > LIMITE_BUFFER_BYTES {
        return Err(ErroFilas::Grande { bytes });
    }
    let tamanho_usize = usize::try_from(tamanho)
        .map_err(|_| ErroFilas::Resposta("tamanho do nome padrão não cabe na memória"))?;
    let mut unidades = vec![0u16; tamanho_usize];
    let mut capacidade = tamanho;
    // Garantia: `unidades` é um vetor vivo e exclusivo com `capacidade` elementos u16; a
    // função escreve no máximo esse número de unidades, incluindo o zero final.
    let segunda =
        unsafe { GetDefaultPrinterW(Some(PWSTR(unidades.as_mut_ptr())), &mut capacidade) };
    if !segunda.as_bool() {
        return Err(ErroFilas::Sistema(codigo(Error::from_thread().code())));
    }
    Ok(Some(texto_de_unidades(&unidades)))
}

/// Chamada em duas etapas: a primeira informa o tamanho, a segunda preenche o buffer.
fn ler_tabela() -> Result<(Vec<u8>, u32), ErroFilas> {
    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
    let mut pedido: u32 = 0;
    let mut contagem: u32 = 0;
    for _ in 0..TENTATIVAS {
        // Garantia: sem buffer (ponteiro nulo, tamanho zero), o sistema só escreve nos dois
        // destinos `u32` locais, que são válidos e exclusivos desta chamada.
        let primeira = unsafe {
            EnumPrintersW(
                flags,
                PCWSTR::null(),
                NIVEL_INFO_2,
                None,
                &mut pedido,
                &mut contagem,
            )
        };
        match primeira {
            // Sem buffer, o sucesso indica que não há filas a ler.
            Ok(()) => return Ok((Vec::new(), 0)),
            Err(erro) if erro.code() == insuficiente() => {}
            Err(erro) => return Err(ErroFilas::Sistema(codigo(erro.code()))),
        }
        if pedido == 0 {
            return Err(ErroFilas::Resposta("tamanho zero para a lista de filas"));
        }
        if pedido > LIMITE_BUFFER_BYTES {
            return Err(ErroFilas::Grande { bytes: pedido });
        }
        let tamanho = usize::try_from(pedido)
            .map_err(|_| ErroFilas::Resposta("tamanho da lista não cabe na memória"))?;
        let mut buffer = vec![0u8; tamanho];
        // Garantia: `buffer` é um slice de bytes vivo e exclusivo, com `len` igual ao tamanho
        // informado; o sistema escreve no máximo esse número de bytes. Os destinos `u32`
        // são locais e válidos. Tamanho e contagem são conferidos em `decodificar`.
        let segunda = unsafe {
            EnumPrintersW(
                flags,
                PCWSTR::null(),
                NIVEL_INFO_2,
                Some(buffer.as_mut_slice()),
                &mut pedido,
                &mut contagem,
            )
        };
        match segunda {
            Ok(()) => return Ok((buffer, contagem)),
            // O novo tamanho pedido já está em `pedido`; a próxima volta o usa.
            Err(erro) if erro.code() == insuficiente() => {}
            Err(erro) => return Err(ErroFilas::Sistema(codigo(erro.code()))),
        }
    }
    Err(ErroFilas::Resposta(
        "a lista de filas mudou durante a leitura",
    ))
}

/// Interpreta a tabela devolvida. Cada registro e cada texto são conferidos contra o buffer.
fn decodificar(
    buffer: &[u8],
    retornadas: u32,
    padrao: Option<&str>,
) -> Result<Vec<Fila>, ErroFilas> {
    let tamanho_registro = std::mem::size_of::<PRINTER_INFO_2W>();
    let total = usize::try_from(retornadas)
        .map_err(|_| ErroFilas::Resposta("contagem de filas inválida"))?;
    let base = buffer.as_ptr() as usize;
    let mut filas = Vec::new();
    for indice in 0..total {
        let inicio = indice
            .checked_mul(tamanho_registro)
            .ok_or(ErroFilas::Resposta("registro de fila fora do buffer"))?;
        let fim = inicio
            .checked_add(tamanho_registro)
            .ok_or(ErroFilas::Resposta("registro de fila fora do buffer"))?;
        let bytes = buffer
            .get(inicio..fim)
            .ok_or(ErroFilas::Resposta("registro de fila fora do buffer"))?;
        // Garantia: `bytes` tem exatamente `size_of::<PRINTER_INFO_2W>()` bytes, vindos do
        // buffer que o sistema preencheu. `read_unaligned` não exige alinhamento. Todos os
        // campos são inteiros ou ponteiros brutos, e nenhum ponteiro é desreferenciado aqui.
        let info: PRINTER_INFO_2W =
            unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<PRINTER_INFO_2W>()) };
        let nome = campo_obrigatorio(buffer, base, info.pPrinterName)?;
        let driver = campo_opcional(buffer, base, info.pDriverName);
        let porta = campo_opcional(buffer, base, info.pPortName);
        filas.push(Fila {
            padrao: e_a_impressora_padrao(padrao, &nome),
            nome,
            driver,
            porta,
            estado: Estado::de_status_e_atributos(info.Status, info.Attributes),
        });
    }
    Ok(filas)
}

/// Texto de um campo que precisa existir (o nome da fila).
fn campo_obrigatorio(buffer: &[u8], base: usize, ponteiro: PWSTR) -> Result<String, ErroFilas> {
    texto_do_campo(buffer, base, ponteiro).ok_or(ErroFilas::Resposta("nome da fila fora do buffer"))
}

/// Texto de um campo opcional: ausente ou fora do buffer vira texto vazio.
fn campo_opcional(buffer: &[u8], base: usize, ponteiro: PWSTR) -> String {
    texto_do_campo(buffer, base, ponteiro).unwrap_or_default()
}

fn texto_do_campo(buffer: &[u8], base: usize, ponteiro: PWSTR) -> Option<String> {
    let endereco = ponteiro.0 as usize;
    if endereco == 0 {
        return None;
    }
    let deslocamento = deslocamento_no_buffer(base, buffer.len(), endereco)?;
    let resto = buffer.get(deslocamento..)?;
    Some(texto_utf16_le(resto))
}

fn insuficiente() -> HRESULT {
    HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0)
}

fn codigo(hresult: HRESULT) -> u32 {
    hresult.0 as u32
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Texto UTF-16 little-endian, terminado em zero.
    fn utf16_com_zero(texto: &str) -> Vec<u8> {
        texto
            .encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(|u| u.to_le_bytes())
            .collect()
    }

    /// Campos de um registro sintético. `None` deixa o ponteiro nulo; `Some(texto)` grava o
    /// texto logo após o registro e aponta para ele. `deslocamento_extra` soma um valor ao
    /// ponteiro do nome, para simular um endereço fora do buffer.
    struct Campos<'a> {
        nome: Option<&'a str>,
        driver: Option<&'a str>,
        porta: Option<&'a str>,
        status: u32,
        atributos: u32,
        deslocamento_extra_do_nome: usize,
    }

    /// Monta um buffer com um registro `PRINTER_INFO_2W` e os textos depois dele.
    /// Os ponteiros usam o endereço real do próprio buffer, que não é movido depois.
    fn montar(campos: &Campos) -> Vec<u8> {
        let tamanho_registro = std::mem::size_of::<PRINTER_INFO_2W>();
        let mut textos: Vec<u8> = Vec::new();
        let mut enderecos = Vec::new();
        for texto in [campos.nome, campos.driver, campos.porta] {
            match texto {
                Some(t) => {
                    enderecos.push(Some(tamanho_registro + textos.len()));
                    textos.extend(utf16_com_zero(t));
                }
                None => enderecos.push(None),
            }
        }
        let mut buffer = vec![0u8; tamanho_registro + textos.len()];
        buffer[tamanho_registro..].copy_from_slice(&textos);
        let base = buffer.as_ptr() as usize;
        let para_ponteiro = |deslocamento: Option<usize>, extra: usize| -> PWSTR {
            match deslocamento {
                Some(d) => PWSTR((base + d + extra) as *mut u16),
                None => PWSTR(std::ptr::null_mut()),
            }
        };
        // Garantia: o registro é zerado, e todos os campos são inteiros ou ponteiros brutos,
        // para os quais o padrão de zeros é válido. Nenhum ponteiro é desreferenciado.
        let mut info: PRINTER_INFO_2W = unsafe { std::mem::zeroed() };
        info.pPrinterName = para_ponteiro(enderecos[0], campos.deslocamento_extra_do_nome);
        info.pDriverName = para_ponteiro(enderecos[1], 0);
        info.pPortName = para_ponteiro(enderecos[2], 0);
        info.Status = campos.status;
        info.Attributes = campos.atributos;
        // Garantia: o registro tem `size_of::<PRINTER_INFO_2W>()` bytes e é lido de volta
        // pela mesma forma em `decodificar`; a cópia de bytes não mexe nos ponteiros.
        let bytes_do_registro = unsafe {
            std::slice::from_raw_parts(
                (&info as *const PRINTER_INFO_2W).cast::<u8>(),
                tamanho_registro,
            )
        };
        buffer[..tamanho_registro].copy_from_slice(bytes_do_registro);
        buffer
    }

    fn campos_padrao() -> Campos<'static> {
        Campos {
            nome: Some("Impressora Lançamento — Escritório"),
            driver: Some("Driver Ação"),
            porta: Some("USB001"),
            status: 0,
            atributos: 0,
            deslocamento_extra_do_nome: 0,
        }
    }

    #[test]
    fn registro_valido_traz_nome_acentuado_com_espaco_driver_e_porta() {
        let buffer = montar(&campos_padrao());
        let filas = decodificar(&buffer, 1, None).unwrap();
        assert_eq!(filas.len(), 1);
        assert_eq!(filas[0].nome, "Impressora Lançamento — Escritório");
        assert_eq!(filas[0].driver, "Driver Ação");
        assert_eq!(filas[0].porta, "USB001");
        assert!(filas[0].estado.pronta());
        assert!(!filas[0].padrao);
    }

    #[test]
    fn registro_marcado_como_padrao_casa_pelo_nome_exato() {
        let buffer = montar(&campos_padrao());
        let padrao = "Impressora Lançamento — Escritório";
        let filas = decodificar(&buffer, 1, Some(padrao)).unwrap();
        assert!(filas[0].padrao);
        let outra = decodificar(&buffer, 1, Some("Impressora Lançamento")).unwrap();
        assert!(!outra[0].padrao);
    }

    #[test]
    fn atributo_usar_offline_chega_ao_estado_da_fila() {
        let mut campos = campos_padrao();
        campos.atributos = 0x0000_0400;
        let buffer = montar(&campos);
        let filas = decodificar(&buffer, 1, None).unwrap();
        assert_eq!(filas[0].estado.condicoes(), &[crate::Condicao::Offline]);
    }

    #[test]
    fn contagem_maior_que_o_buffer_vira_erro_de_resposta() {
        let buffer = montar(&campos_padrao());
        let erro = decodificar(&buffer, 2, None).unwrap_err();
        assert!(matches!(erro, ErroFilas::Resposta(_)), "{erro:?}");
    }

    #[test]
    fn nome_apontando_para_fora_do_buffer_vira_erro() {
        let mut campos = campos_padrao();
        campos.deslocamento_extra_do_nome = 10_000;
        let buffer = montar(&campos);
        let erro = decodificar(&buffer, 1, None).unwrap_err();
        assert_eq!(erro, ErroFilas::Resposta("nome da fila fora do buffer"));
    }

    #[test]
    fn nome_nulo_vira_erro() {
        let mut campos = campos_padrao();
        campos.nome = None;
        let buffer = montar(&campos);
        let erro = decodificar(&buffer, 1, None).unwrap_err();
        assert_eq!(erro, ErroFilas::Resposta("nome da fila fora do buffer"));
    }

    #[test]
    fn driver_e_porta_nulos_viram_texto_vazio() {
        let mut campos = campos_padrao();
        campos.driver = None;
        campos.porta = None;
        let buffer = montar(&campos);
        let filas = decodificar(&buffer, 1, None).unwrap();
        assert_eq!(filas[0].nome, "Impressora Lançamento — Escritório");
        assert_eq!(filas[0].driver, "");
        assert_eq!(filas[0].porta, "");
    }
}
