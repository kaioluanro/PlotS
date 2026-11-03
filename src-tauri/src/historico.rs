//! O histórico local: quem lê e apaga para a interface, e o diário de cada envio, que junta como
//! cada impressora terminou em um só registro. Falha ao gravar nunca chega à impressão, e os detalhes
//! de um arquivo danificado não são mostrados à interface.

use crate::impressoras::GRAVACAO_DE_PERFIS;
use crate::trabalhos::{ComparacaoDoResumo, Desfecho, FimDoTrabalho, Gerente};
use plots_perfis::historico::{self, RegistroDeEnvio, ResultadoDoEnvio, TempoDaImpressora};
use plots_perfis::ErroPerfis;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

/// Quanto o fechamento do programa espera os trabalhos contarem como foi.
const PRAZO_AO_ENCERRAR: Duration = Duration::from_millis(1_500);

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FalhaHistorico {
    Leitura,
    Gravacao,
}

/// Velocidade de uma impressora: só existe com folhas medidas suficientes.
#[derive(Serialize)]
pub struct VelocidadeDaFila {
    pub impressora: String,
    pub folhas_por_minuto: f64,
    /// A mesma impressora pode aparecer duas vezes: uma em cada rota.
    pub automatico: bool,
}

#[derive(Serialize)]
pub struct ListaHistorico {
    pub registros: Vec<RegistroDeEnvio>,
    pub ignorados: usize,
    pub velocidades: Vec<VelocidadeDaFila>,
}

fn caminho() -> Result<PathBuf, FalhaHistorico> {
    if let Some(caminho) = std::env::var_os("PLOTS_HISTORICO").filter(|valor| !valor.is_empty()) {
        return Ok(PathBuf::from(caminho));
    }
    historico::caminho_padrao_do_historico().map_err(|_| FalhaHistorico::Leitura)
}

fn velocidades_de(registros: &[RegistroDeEnvio]) -> Vec<VelocidadeDaFila> {
    let medidas: BTreeSet<(&str, bool)> = registros
        .iter()
        .flat_map(|registro| &registro.tempos)
        .map(|tempo| (tempo.impressora.as_str(), tempo.automatico))
        .collect();
    medidas
        .into_iter()
        .filter_map(|(nome, automatico)| {
            let folhas_por_minuto =
                historico::velocidade_da_impressora(registros, nome, automatico)?;
            Some(VelocidadeDaFila {
                impressora: nome.to_owned(),
                folhas_por_minuto,
                automatico,
            })
        })
        .collect()
}

#[tauri::command(async)]
pub fn listar_historico() -> Result<ListaHistorico, FalhaHistorico> {
    let lido = historico::carregar_historico(&caminho()?).map_err(|_| FalhaHistorico::Leitura)?;
    let registros = lido.registros();
    Ok(ListaHistorico {
        velocidades: velocidades_de(&registros),
        ignorados: lido.ignorados(),
        registros,
    })
}

#[tauri::command(async)]
pub fn apagar_historico() -> Result<(), FalhaHistorico> {
    historico::apagar_historico(&caminho()?).map_err(|_| FalhaHistorico::Gravacao)
}

/// A escolha de guardar o histórico fica no mesmo arquivo dos perfis. Sem arquivo, vale ligada.
fn ler_escolha(perfis: Result<PathBuf, ErroPerfis>) -> Result<bool, FalhaHistorico> {
    let caminho = perfis.map_err(|_| FalhaHistorico::Leitura)?;
    let lidos = plots_perfis::carregar(&caminho).map_err(|_| FalhaHistorico::Leitura)?;
    Ok(lidos.guardar_historico())
}

/// Um arquivo de perfis que não abriu não é substituído: a escolha fica como estava.
fn gravar_escolha(perfis: Result<PathBuf, ErroPerfis>, ligado: bool) -> Result<(), FalhaHistorico> {
    let _vez = GRAVACAO_DE_PERFIS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let caminho = perfis.map_err(|_| FalhaHistorico::Gravacao)?;
    let mut lidos = plots_perfis::carregar(&caminho).map_err(|_| FalhaHistorico::Gravacao)?;
    lidos.definir_guardar_historico(ligado);
    plots_perfis::salvar(&caminho, &lidos).map_err(|_| FalhaHistorico::Gravacao)
}

#[tauri::command(async)]
pub fn ler_guardar_historico() -> Result<bool, FalhaHistorico> {
    ler_escolha(crate::impressoras::caminho_dos_perfis_do_sistema())
}

#[tauri::command(async)]
pub fn definir_guardar_historico(ligado: bool) -> Result<(), FalhaHistorico> {
    gravar_escolha(crate::impressoras::caminho_dos_perfis_do_sistema(), ligado)
}

/// Compara as impressoras pelo histórico, na rota dada. Sem arquivo, com arquivo danificado ou sem
/// dados de todas, não há o que dizer, e nada aparece na confirmação.
pub fn comparacao_do_envio(impressoras: &[String], automatico: bool) -> Option<ComparacaoDoResumo> {
    let registros = historico::carregar_historico(&caminho().ok()?)
        .ok()?
        .registros();
    comparar_pelos_registros(&registros, impressoras, automatico)
}

fn comparar_pelos_registros(
    registros: &[RegistroDeEnvio],
    impressoras: &[String],
    automatico: bool,
) -> Option<ComparacaoDoResumo> {
    let medidas: Vec<(String, Option<f64>)> = impressoras
        .iter()
        .map(|nome| {
            (
                nome.clone(),
                historico::velocidade_da_impressora(registros, nome, automatico),
            )
        })
        .collect();
    let comparacao = historico::comparar_velocidades(&medidas)?;
    Some(ComparacaoDoResumo {
        rapida: comparacao.rapida,
        lenta: comparacao.lenta,
        razao: comparacao.razao,
    })
}

/// O que se sabe do envio antes de cada impressora terminar.
#[derive(Clone, Debug)]
pub struct CabecalhoDoEnvio {
    pub arquivo: String,
    pub intervalo: Option<(usize, usize)>,
    pub divisao: bool,
    pub lotes_confirmados: usize,
}

struct Parte {
    impressora: String,
    simulada: bool,
    desfecho: Desfecho,
}

struct EstadoDoDiario {
    impressoras: Vec<String>,
    pendentes: BTreeSet<String>,
    partes: Vec<Parte>,
    gravado: bool,
}

type Gravar = Box<dyn Fn(RegistroDeEnvio) + Send + Sync>;

/// Um envio em andamento, em uma ou várias impressoras. Vira um registro uma vez só: quando a última
/// termina, ou quando o programa fecha com alguma ainda em curso.
pub struct Diario {
    cabecalho: CabecalhoDoEnvio,
    data: u64,
    estado: Mutex<EstadoDoDiario>,
    gravar: Gravar,
}

impl Diario {
    fn novo(
        cabecalho: CabecalhoDoEnvio,
        impressoras: Vec<String>,
        data: u64,
        gravar: Gravar,
    ) -> Self {
        Self {
            cabecalho,
            data,
            estado: Mutex::new(EstadoDoDiario {
                pendentes: impressoras.iter().cloned().collect(),
                impressoras,
                partes: Vec::new(),
                gravado: false,
            }),
            gravar,
        }
    }

    fn com_estado<T>(&self, acao: impl FnOnce(&mut EstadoDoDiario) -> T) -> T {
        let mut estado = self.estado.lock().unwrap_or_else(PoisonError::into_inner);
        acao(&mut estado)
    }

    /// Grava se não falta ninguém, ou se `mesmo_faltando`. O registro sai fora da trava.
    fn gravar_se_pronto(&self, mesmo_faltando: bool) {
        let registro = self.com_estado(|estado| {
            let pronto = estado.pendentes.is_empty() || mesmo_faltando;
            if estado.gravado || !pronto || estado.impressoras.is_empty() {
                return None;
            }
            estado.gravado = true;
            Some(compor_registro(&self.cabecalho, self.data, estado))
        });
        if let Some(registro) = registro {
            (self.gravar)(registro);
        }
    }

    /// O que a thread do trabalho chama ao acabar.
    pub fn fim_da_parte(diario: &Arc<Self>, impressora: &str, simulada: bool) -> FimDoTrabalho {
        let diario = Arc::clone(diario);
        let impressora = impressora.to_owned();
        Box::new(move |desfecho| {
            diario.com_estado(|estado| {
                estado.pendentes.remove(&impressora);
                estado.partes.push(Parte {
                    impressora,
                    simulada,
                    desfecho,
                });
            });
            diario.gravar_se_pronto(false);
        })
    }

    /// Esta impressora não chegou a receber o trabalho.
    pub fn desistir(&self, impressora: &str) {
        self.com_estado(|estado| {
            estado.pendentes.remove(impressora);
            estado.impressoras.retain(|nome| nome != impressora);
        });
        self.gravar_se_pronto(false);
    }

    fn gravado(&self) -> bool {
        self.com_estado(|estado| estado.gravado)
    }
}

fn numero(valor: usize) -> u64 {
    u64::try_from(valor).unwrap_or(u64::MAX)
}

/// Erro pesa mais que cancelado, que pesa mais que incompleto: o registro diz o pior que houve.
fn resultado_do_envio(estado: &EstadoDoDiario) -> ResultadoDoEnvio {
    let resultados = || estado.partes.iter().map(|parte| parte.desfecho.resultado);
    if resultados().any(|resultado| resultado == ResultadoDoEnvio::ComErro) {
        ResultadoDoEnvio::ComErro
    } else if resultados().any(|resultado| resultado == ResultadoDoEnvio::Cancelado) {
        ResultadoDoEnvio::Cancelado
    } else if !estado.pendentes.is_empty()
        || resultados().any(|resultado| resultado == ResultadoDoEnvio::Incompleto)
    {
        ResultadoDoEnvio::Incompleto
    } else {
        ResultadoDoEnvio::Concluido
    }
}

/// O tempo fingido da simulada não diz nada de impressora nenhuma, e por isso não entra.
fn tempos_das_partes(partes: &[Parte]) -> Vec<TempoDaImpressora> {
    partes
        .iter()
        .filter(|parte| !parte.simulada)
        .flat_map(|parte| {
            [
                (parte.desfecho.medida, false),
                (parte.desfecho.medida_automatica, true),
            ]
            .into_iter()
            .filter(|(medida, _)| medida.milissegundos > 0)
            .map(|(medida, automatico)| TempoDaImpressora {
                impressora: parte.impressora.clone(),
                milissegundos: medida.milissegundos,
                folhas: medida.folhas,
                resultado: parte.desfecho.resultado,
                automatico,
            })
        })
        .collect()
}

fn compor_registro(
    cabecalho: &CabecalhoDoEnvio,
    data: u64,
    estado: &EstadoDoDiario,
) -> RegistroDeEnvio {
    let desfechos = || estado.partes.iter().map(|parte| parte.desfecho);
    // Na divisão, cada uma tem a sua parte; no espelho, cada uma imprime tudo.
    let (lotes, faltando, paginas) = if cabecalho.divisao {
        let lotes = numero(cabecalho.lotes_confirmados);
        let feitos = desfechos().fold(0u64, |soma, item| soma.saturating_add(item.lotes_feitos));
        let paginas = desfechos().fold(0u64, |soma, item| soma.saturating_add(item.paginas));
        (lotes, lotes.saturating_sub(feitos), paginas)
    } else {
        let lotes = desfechos().map(|item| item.lotes).max().unwrap_or(0);
        let faltando = if estado.pendentes.is_empty() {
            desfechos()
                .map(|item| item.lotes.saturating_sub(item.lotes_feitos))
                .max()
                .unwrap_or(0)
        } else {
            lotes
        };
        let paginas = desfechos().map(|item| item.paginas).max().unwrap_or(0);
        (lotes, faltando, paginas)
    };
    RegistroDeEnvio {
        arquivo: cabecalho.arquivo.clone(),
        data,
        impressoras: estado.impressoras.clone(),
        intervalo: cabecalho
            .intervalo
            .map(|(inicio, fim)| (numero(inicio), numero(fim))),
        paginas,
        lotes,
        resultado: resultado_do_envio(estado),
        lotes_faltando: faltando,
        tempos: tempos_das_partes(&estado.partes),
    }
}

/// Os envios ainda sem registro, para o fechamento do programa não perder nenhum.
#[derive(Default)]
pub struct Diarios(Mutex<Vec<Arc<Diario>>>);

impl Diarios {
    /// Um diário novo. A escolha de guardar vale como está agora, até o fim deste envio. Sem conseguir
    /// lê-la, nada é registrado: quem desligou não pode voltar a ter envios na lista por um arquivo
    /// que não abriu.
    pub fn abrir(&self, cabecalho: CabecalhoDoEnvio, impressoras: Vec<String>) -> Arc<Diario> {
        let guardar =
            ler_escolha(crate::impressoras::caminho_dos_perfis_do_sistema()).unwrap_or(false);
        self.abrir_com_escolha(cabecalho, impressoras, guardar, caminho())
    }

    /// Uma falha ao gravar fica só aqui: o envio segue.
    fn abrir_com_escolha(
        &self,
        cabecalho: CabecalhoDoEnvio,
        impressoras: Vec<String>,
        guardar: bool,
        destino: Result<PathBuf, FalhaHistorico>,
    ) -> Arc<Diario> {
        let data = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |tempo| tempo.as_secs());
        let destino = destino.ok().filter(|_| guardar);
        let gravar: Gravar = Box::new(move |registro| {
            if let Some(caminho) = &destino {
                let _ = historico::acrescentar_ao_historico(caminho, registro);
            }
        });
        self.guardar(Diario::novo(cabecalho, impressoras, data, gravar))
    }

    fn guardar(&self, diario: Diario) -> Arc<Diario> {
        let diario = Arc::new(diario);
        let mut lista = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        lista.retain(|existente| !existente.gravado());
        lista.push(Arc::clone(&diario));
        diario
    }

    /// Grava o que estiver aberto, como está.
    fn descarregar(&self) {
        let lista = std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner));
        for diario in lista {
            diario.gravar_se_pronto(true);
        }
    }
}

/// O programa está fechando: os trabalhos contam como foram, e quem não respondeu a tempo entra como
/// incompleto. O que já está na fila continua lá.
pub fn ao_encerrar(app: &AppHandle) {
    if let Some(gerente) = app.try_state::<Gerente>() {
        gerente.soltar_todos_e_esperar(PRAZO_AO_ENCERRAR);
    }
    if let Some(diarios) = app.try_state::<Diarios>() {
        diarios.descarregar();
    }
}

#[cfg(test)]
#[path = "historico_testes.rs"]
mod testes;
