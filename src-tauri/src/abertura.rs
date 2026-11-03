//! Tela de abertura: uma janela pequena enquanto o programa prepara tudo, e so depois a janela
//! principal. O carregamento roda em threads proprias; a janela principal nunca fica sem aparecer.

use crate::documento;
use crate::impressoras::{self, CodigoAviso, ListaImpressoras};
use crate::lote;
use plots_windows::{caminhos_da_biblioteca, escolher_biblioteca, VARIAVEL_PASTA_BIBLIOTECA};
use serde::Serialize;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, Runtime};

/// Tempo minimo da tela de abertura, para ela nao piscar.
pub const MINIMO: Duration = Duration::from_millis(800);
/// Tempo maximo: passado ele, a janela principal abre mesmo com o carregamento incompleto.
pub const LIMITE: Duration = Duration::from_secs(10);
const PASSO: Duration = Duration::from_millis(20);
/// Quanto a interface espera, no maximo, pela passagem depois de avisar que montou.
const ESPERA_DA_INTERFACE: Duration = Duration::from_secs(15);
const TOTAL_DE_ETAPAS: u8 = 3;

/// O que o carregamento esta fazendo; a interface traduz o codigo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Etapa {
    Impressoras,
    Documento,
    ComponenteImpressao,
    Pronto,
}

impl Etapa {
    fn codigo(self) -> &'static str {
        match self {
            Etapa::Impressoras => "impressoras",
            Etapa::Documento => "documento",
            Etapa::ComponenteImpressao => "componente_impressao",
            Etapa::Pronto => "pronto",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodigoAvisoAbertura {
    Filas,
    Perfis,
    Documento,
    ComponenteImpressao,
    Limite,
}

/// Algo que falhou ou ficou pendente na abertura. A interface traduz o codigo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AvisoAbertura {
    pub codigo: CodigoAvisoAbertura,
    /// Nome do arquivo, ou a etapa que ficou pendente no aviso de limite.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detalhe: Option<String>,
}

#[derive(Debug)]
pub struct Estado {
    etapa: Etapa,
    concluidas: u8,
    carga_pronta: bool,
    montou: bool,
    passou: bool,
    avisos: Vec<AvisoAbertura>,
}

impl Default for Estado {
    fn default() -> Self {
        Self {
            etapa: Etapa::Impressoras,
            concluidas: 0,
            carga_pronta: false,
            montou: false,
            passou: false,
            avisos: Vec::new(),
        }
    }
}

impl Estado {
    /// Guarda o aviso de que o prazo acabou, dizendo em que etapa o carregamento estava.
    fn registrar_limite(&mut self) {
        self.avisos.push(AvisoAbertura {
            codigo: CodigoAvisoAbertura::Limite,
            detalhe: Some(self.etapa.codigo().to_owned()),
        });
    }

    /// Guarda o aviso de que a biblioteca de impressao nao foi encontrada.
    fn registrar_componente(&mut self, presente: bool) {
        if !presente {
            self.avisos.push(AvisoAbertura {
                codigo: CodigoAvisoAbertura::ComponenteImpressao,
                detalhe: None,
            });
        }
    }

    /// A passagem so existe depois de a janela principal ser mostrada, e traz os avisos lidos no
    /// mesmo instante, sob a mesma trava.
    fn passagem_pronta(&self) -> Option<Passagem> {
        self.passou.then(|| Passagem {
            avisos: self.avisos.clone(),
        })
    }
}

/// Andamento mostrado na tela de abertura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Andamento {
    pub etapa: Etapa,
    pub concluidas: u8,
    pub total: u8,
}

/// O que a janela principal recebe ao ser mostrada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Passagem {
    pub avisos: Vec<AvisoAbertura>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decisao {
    Esperar(Duration),
    /// `por_limite`: o prazo acabou com o carregamento ainda incompleto.
    Passar {
        por_limite: bool,
    },
}

/// Decide, pelo tempo decorrido, se ja e hora de mostrar a janela principal.
pub fn decidir(decorrido: Duration, carga_pronta: bool, interface_montou: bool) -> Decisao {
    if decorrido >= LIMITE {
        return Decisao::Passar {
            por_limite: !carga_pronta,
        };
    }
    if carga_pronta && interface_montou && decorrido >= MINIMO {
        return Decisao::Passar { por_limite: false };
    }
    Decisao::Esperar(PASSO.min(LIMITE.saturating_sub(decorrido)))
}

/// Avisos que uma leitura da lista de impressoras deixa para a abertura.
pub fn avisos_da_lista(lista: &ListaImpressoras) -> Vec<AvisoAbertura> {
    lista
        .avisos
        .iter()
        .filter_map(|aviso| {
            let codigo = match aviso.codigo {
                CodigoAviso::Filas => CodigoAvisoAbertura::Filas,
                CodigoAviso::PerfisPasta
                | CodigoAviso::PerfisLeitura
                | CodigoAviso::PerfisGrande
                | CodigoAviso::PerfisInvalido
                | CodigoAviso::PerfisVersao => CodigoAvisoAbertura::Perfis,
                CodigoAviso::ModeloNaoGravado => return None,
            };
            Some(AvisoAbertura {
                codigo,
                detalhe: None,
            })
        })
        .collect()
}

/// Verifica, sem carregar, se a biblioteca de impressao esta onde a impressao a procura: na pasta
/// indicada pela variavel de ambiente ou, sem ela, ao lado do executavel. Reaproveita a busca da
/// impressao, para o aviso nunca discordar do que a impressao vai encontrar.
pub fn componente_de_impressao_presente(
    pasta_indicada: Option<&OsStr>,
    pasta_do_executavel: Option<&Path>,
    existe: impl Fn(&Path) -> bool,
) -> bool {
    caminhos_da_biblioteca(pasta_indicada, pasta_do_executavel)
        .and_then(|caminhos| escolher_biblioteca(caminhos, existe))
        .is_ok()
}

fn com_estado<R: Runtime, T>(app: &AppHandle<R>, acao: impl FnOnce(&mut Estado) -> T) -> Option<T> {
    let estado = app.try_state::<Mutex<Estado>>()?;
    let mut trava = estado.lock().unwrap_or_else(PoisonError::into_inner);
    Some(acao(&mut trava))
}

pub fn andamento<R: Runtime>(app: &AppHandle<R>) -> Andamento {
    com_estado(app, |estado| Andamento {
        etapa: estado.etapa,
        concluidas: estado.concluidas,
        total: TOTAL_DE_ETAPAS,
    })
    .unwrap_or(Andamento {
        etapa: Etapa::Pronto,
        concluidas: TOTAL_DE_ETAPAS,
        total: TOTAL_DE_ETAPAS,
    })
}

/// Marca a etapa anterior como concluida e passa para `etapa`, guardando os avisos que ela deixou.
fn avancar<R: Runtime>(app: &AppHandle<R>, etapa: Etapa, avisos: Vec<AvisoAbertura>) {
    com_estado(app, |estado| {
        estado.concluidas = estado.concluidas.saturating_add(1).min(TOTAL_DE_ETAPAS);
        estado.etapa = etapa;
        estado.avisos.extend(avisos);
        if etapa == Etapa::Pronto {
            estado.carga_pronta = true;
        }
    });
}

/// Atraso artificial para o teste de uso do limite; so existe em compilacao de depuracao.
#[cfg(debug_assertions)]
fn atraso_de_teste() -> Duration {
    std::env::var("PLOTS_ABERTURA_ATRASO_MS")
        .ok()
        .and_then(|valor| valor.trim().parse::<u64>().ok())
        .map_or(Duration::ZERO, Duration::from_millis)
}

#[cfg(not(debug_assertions))]
fn atraso_de_teste() -> Duration {
    Duration::ZERO
}

/// Perfis e impressoras, o primeiro PDF recebido e a biblioteca de impressao. Serve para falhar
/// cedo e avisar; a janela principal le tudo de novo quando precisar.
fn carregar<R: Runtime>(app: &AppHandle<R>) {
    // So olha se o arquivo existe, e vem primeiro: se o prazo estourar numa etapa lenta, o aviso
    // da biblioteca ausente ja esta guardado e sai junto com o aviso de limite.
    let indicada = std::env::var_os(VARIAVEL_PASTA_BIBLIOTECA);
    let executavel = std::env::current_exe().ok();
    let presente = componente_de_impressao_presente(
        indicada.as_deref(),
        executavel.as_deref().and_then(Path::parent),
        Path::is_file,
    );
    com_estado(app, |estado| estado.registrar_componente(presente));

    std::thread::sleep(atraso_de_teste());
    let lista = impressoras::ler_do_sistema();
    avancar(app, Etapa::Documento, avisos_da_lista(&lista));

    let recebidos = lote::dos_argumentos(
        std::env::args_os()
            .skip(1)
            .map(|argumento| argumento.to_string_lossy().into_owned()),
    );
    let primeiro = recebidos.caminhos.first();
    let mut avisos = Vec::new();
    if let Some(caminho) = primeiro {
        if let Err(erro) = documento::contar_paginas(caminho) {
            avisos.push(AvisoAbertura {
                codigo: CodigoAvisoAbertura::Documento,
                detalhe: Some(erro.nome),
            });
        }
    }
    avancar(app, Etapa::ComponenteImpressao, avisos);

    avancar(app, Etapa::Pronto, Vec::new());
}

/// Fecha a abertura e mostra a janela principal. Cada passo que o sistema recusar e ignorado: a
/// janela principal precisa aparecer de qualquer jeito.
fn passar<R: Runtime>(app: &AppHandle<R>, por_limite: bool) {
    let situacao = com_estado(app, |estado| {
        if por_limite {
            estado.registrar_limite();
        }
        (estado.etapa, estado.carga_pronta, estado.montou)
    });
    // A principal aparece antes de a abertura sumir, para nunca ficar sem janela a vista.
    if let Some(principal) = app.get_webview_window("main") {
        let _ = principal.show();
        let _ = principal.set_focus();
    }
    if let Some(abertura) = app.get_webview_window("abertura") {
        let _ = abertura.destroy();
    }
    com_estado(app, |estado| estado.passou = true);
    registrar_passagem(por_limite, situacao);
}

/// Rastro da passagem para investigar demoras; so existe em compilacao de depuracao.
#[cfg(debug_assertions)]
fn registrar_passagem(por_limite: bool, situacao: Option<(Etapa, bool, bool)>) {
    use std::io::Write;
    let _ = writeln!(
        std::io::stderr(),
        "abertura: passou por_limite={por_limite} situacao(etapa, carga_pronta, montou)={situacao:?}"
    );
}

#[cfg(not(debug_assertions))]
fn registrar_passagem(_por_limite: bool, _situacao: Option<(Etapa, bool, bool)>) {}

fn supervisionar<R: Runtime>(app: &AppHandle<R>, inicio: Instant) {
    loop {
        if app.get_webview_window("abertura").is_none() {
            passar(app, false);
            return;
        }
        let (pronta, montou) =
            com_estado(app, |estado| (estado.carga_pronta, estado.montou)).unwrap_or((true, true));
        match decidir(inicio.elapsed(), pronta, montou) {
            Decisao::Esperar(restante) => std::thread::sleep(restante),
            Decisao::Passar { por_limite } => {
                passar(app, por_limite);
                return;
            }
        }
    }
}

/// Chamada no `setup`. Se algo falhar ao iniciar, a janela principal abre na hora.
pub fn iniciar<R: Runtime>(app: &AppHandle<R>) {
    let inicio = Instant::now();
    if app.get_webview_window("abertura").is_none() {
        passar(app, false);
        return;
    }
    let do_prazo = app.clone();
    let prazo = std::thread::Builder::new()
        .name("abertura-prazo".into())
        .spawn(move || supervisionar(&do_prazo, inicio));
    if prazo.is_err() {
        passar(app, false);
        return;
    }
    // Se a carga nao iniciar, o prazo mostra a janela principal ao fim do limite, com o aviso.
    let da_carga = app.clone();
    let _ = std::thread::Builder::new()
        .name("abertura-carga".into())
        .spawn(move || carregar(&da_carga));
}

/// A interface montou: espera a passagem e devolve os avisos. Nunca espera para sempre.
pub fn marcar_montou_e_esperar<R: Runtime>(app: &AppHandle<R>) -> Passagem {
    com_estado(app, |estado| estado.montou = true);
    let inicio = Instant::now();
    loop {
        // Uma leitura so: o aviso de limite e o fim da passagem nunca se separam.
        let lido = com_estado(app, |estado| {
            (
                estado.passagem_pronta(),
                Passagem {
                    avisos: estado.avisos.clone(),
                },
            )
        });
        let Some((pronta, parcial)) = lido else {
            return Passagem { avisos: Vec::new() };
        };
        if let Some(passagem) = pronta {
            return passagem;
        }
        if inicio.elapsed() >= ESPERA_DA_INTERFACE {
            return parcial;
        }
        std::thread::sleep(PASSO);
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::impressoras::Aviso;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn carga_rapida_ainda_espera_o_minimo() {
        assert_eq!(decidir(ms(10), true, true), Decisao::Esperar(PASSO));
        assert_eq!(decidir(ms(799), true, true), Decisao::Esperar(PASSO));
        assert_eq!(
            decidir(MINIMO, true, true),
            Decisao::Passar { por_limite: false }
        );
    }

    #[test]
    fn sem_carga_ou_sem_interface_continua_esperando() {
        assert!(matches!(
            decidir(ms(2_000), false, true),
            Decisao::Esperar(_)
        ));
        assert!(matches!(
            decidir(ms(2_000), true, false),
            Decisao::Esperar(_)
        ));
    }

    #[test]
    fn no_limite_passa_mesmo_incompleto_e_marca_o_motivo() {
        assert_eq!(
            decidir(LIMITE, false, false),
            Decisao::Passar { por_limite: true }
        );
        assert_eq!(
            decidir(LIMITE, true, false),
            Decisao::Passar { por_limite: false }
        );
        assert!(matches!(
            decidir(ms(9_999), false, false),
            Decisao::Esperar(_)
        ));
    }

    #[test]
    fn a_espera_nunca_passa_do_limite_e_tempos_extremos_nao_quebram() {
        assert_eq!(decidir(ms(9_990), false, false), Decisao::Esperar(ms(10)));
        assert_eq!(
            decidir(Duration::MAX, false, false),
            Decisao::Passar { por_limite: true }
        );
        assert_eq!(
            decidir(Duration::ZERO, false, false),
            Decisao::Esperar(PASSO)
        );
    }

    #[test]
    fn avisos_da_lista_traduz_filas_e_perfis_e_ignora_gravacao() {
        let aviso = |codigo| Aviso {
            codigo,
            detalhe: None,
            caminho: None,
        };
        let lista = ListaImpressoras {
            impressoras: Vec::new(),
            avisos: vec![
                aviso(CodigoAviso::Filas),
                aviso(CodigoAviso::PerfisInvalido),
                aviso(CodigoAviso::ModeloNaoGravado),
            ],
        };
        let codigos: Vec<_> = avisos_da_lista(&lista)
            .into_iter()
            .map(|a| a.codigo)
            .collect();
        assert_eq!(
            codigos,
            vec![CodigoAvisoAbertura::Filas, CodigoAvisoAbertura::Perfis]
        );
    }

    #[test]
    fn limite_estourado_com_componente_ausente_traz_os_dois_avisos() {
        let mut estado = Estado::default();
        // A verificacao do componente e feita no inicio, antes de qualquer etapa lenta.
        estado.registrar_componente(false);
        estado.registrar_limite();
        estado.passou = true;
        let codigos: Vec<_> = estado
            .passagem_pronta()
            .expect("passagem pronta")
            .avisos
            .into_iter()
            .map(|aviso| aviso.codigo)
            .collect();
        assert_eq!(
            codigos,
            vec![
                CodigoAvisoAbertura::ComponenteImpressao,
                CodigoAvisoAbertura::Limite
            ]
        );
    }

    #[test]
    fn componente_presente_nao_gera_aviso() {
        let mut estado = Estado::default();
        estado.registrar_componente(true);
        assert!(estado.avisos.is_empty());
    }

    #[test]
    fn avisos_e_passagem_saem_juntos_da_mesma_leitura() {
        let mut estado = Estado::default();
        assert_eq!(
            estado.passagem_pronta(),
            None,
            "antes de passar nao ha passagem"
        );
        estado.registrar_limite();
        // O aviso ja foi guardado, mas a janela ainda nao foi mostrada.
        assert_eq!(estado.passagem_pronta(), None);
        estado.passou = true;
        let passagem = estado.passagem_pronta().expect("passagem pronta");
        assert_eq!(passagem.avisos.len(), 1);
        assert_eq!(passagem.avisos[0].codigo, CodigoAvisoAbertura::Limite);
        assert_eq!(passagem.avisos[0].detalhe.as_deref(), Some("impressoras"));
    }

    #[test]
    fn componente_de_impressao_usa_a_mesma_busca_da_impressao() {
        let pasta_exe = Path::new("C:/programa");
        let na_pasta_do_exe = |c: &Path| c == Path::new("C:/programa/pdfium.dll");
        assert!(componente_de_impressao_presente(
            None,
            Some(pasta_exe),
            na_pasta_do_exe
        ));
        assert!(!componente_de_impressao_presente(
            None,
            Some(pasta_exe),
            |_| false
        ));
        assert!(!componente_de_impressao_presente(None, None, |_| true));

        // A pasta indicada vale no lugar do executavel, direto ou em bin.
        let indicada = OsStr::new("C:/extraido");
        let em_bin = |c: &Path| c == Path::new("C:/extraido/bin/pdfium.dll");
        assert!(componente_de_impressao_presente(
            Some(indicada),
            Some(pasta_exe),
            em_bin
        ));
        assert!(!componente_de_impressao_presente(
            Some(indicada),
            Some(pasta_exe),
            na_pasta_do_exe
        ));
        // Caminho relativo na variavel e recusado pela impressao, entao tambem avisa aqui.
        assert!(!componente_de_impressao_presente(
            Some(OsStr::new("relativa")),
            Some(pasta_exe),
            |_| true
        ));
        // Variavel vazia conta como ausente.
        assert!(componente_de_impressao_presente(
            Some(OsStr::new("")),
            Some(pasta_exe),
            na_pasta_do_exe
        ));
    }
}
