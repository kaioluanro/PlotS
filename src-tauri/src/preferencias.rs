//! Preferencias do driver de cada fila: abrir a janela dele, guardar o que a pessoa aceitou no
//! arquivo de perfis e voltar ao padrao. O padrao do Windows nunca muda.

use crate::impressoras::{GRAVACAO_DE_PERFIS, VARIAVEL_PERFIS};
use plots_perfis::{AjusteRecusado, ErroPerfis, Perfis};
use plots_windows::{
    Bandeja, Cor, ErroPreferencias, Escolhas, FonteDeAjustes, OpcoesDaFila, Resolucao,
    RespostaDaJanela,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;
use tauri::async_runtime::Receiver;
use tauri::{AppHandle, Emitter, Manager, Runtime, Window};

/// A interface fica sabendo por este evento que o driver nao aceitou o ajuste de uma fila.
pub const EVENTO_DE_RECUSA: &str = "preferencias-recusada";

/// Filas com a janela do driver aberta agora.
static ABERTAS: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Filas em que o driver acabou de recusar o ajuste, ate o cartao do trabalho contar.
static RECUSAS: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Em que pe esta o fechamento que espera a janela do driver.
static FECHO: Mutex<Fecho> = Mutex::new(Fecho::Livre);

/// Quanto o fechamento espera a janela do driver sair antes de seguir sem ela.
const PRAZO_DA_JANELA: Duration = Duration::from_secs(2);

/// Motivo de uma falha, em forma de codigo; a interface traduz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodigoFalha {
    FilaSumiu,
    NaoAbriu,
    NaoGravou,
    Grande,
    /// O driver não respondeu com as opções ou não aceitou as escolhidas.
    OpcaoRecusada,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FalhaPreferencias {
    pub codigo: CodigoFalha,
    pub fila: String,
}

impl FalhaPreferencias {
    fn nova(codigo: CodigoFalha, fila: &str) -> Self {
        Self {
            codigo,
            fila: fila.to_owned(),
        }
    }
}

/// Como terminou o pedido de abrir a janela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FimDaJanela {
    Salvo,
    Cancelado,
    /// A janela desta fila ja estava aberta: nenhuma outra foi aberta.
    JaAberta,
}

/// O que a lista precisa saber: quem tem ajuste e de quem o ajuste estava corrompido.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EstadoDasPreferencias {
    pub salvas: Vec<String>,
    pub ignoradas: Vec<String>,
    /// Filas cujo ajuste diz duas faces no campo publico. O envio continua em uma face so.
    #[serde(rename = "duasFaces")]
    pub duas_faces: Vec<String>,
}

/// Bandeja que o driver declara para a fila.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BandejaDaImpressora {
    pub valor: i16,
    pub nome: String,
}

/// Par de resolução que o driver declara, em dpi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolucaoDaImpressora {
    pub x: i32,
    pub y: i32,
}

/// Cor que o driver declara.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorDaImpressora {
    Colorida,
    Monocromatica,
}

/// O que a pessoa escolheu; `null` deixa o campo como o driver o entregou.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EscolhasDaImpressora {
    pub bandeja: Option<i16>,
    pub resolucao: Option<ResolucaoDaImpressora>,
    pub cor: Option<CorDaImpressora>,
}

/// O que o driver declara para a fila, com os valores que o ajuste atual usa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpcoesDaImpressora {
    pub bandejas: Vec<BandejaDaImpressora>,
    pub resolucoes: Vec<ResolucaoDaImpressora>,
    pub cor: Option<CorDaImpressora>,
    pub atual: EscolhasDaImpressora,
}

impl From<Cor> for CorDaImpressora {
    fn from(cor: Cor) -> Self {
        match cor {
            Cor::Colorida => Self::Colorida,
            Cor::Monocromatica => Self::Monocromatica,
        }
    }
}

impl From<CorDaImpressora> for Cor {
    fn from(cor: CorDaImpressora) -> Self {
        match cor {
            CorDaImpressora::Colorida => Self::Colorida,
            CorDaImpressora::Monocromatica => Self::Monocromatica,
        }
    }
}

impl From<Bandeja> for BandejaDaImpressora {
    fn from(bandeja: Bandeja) -> Self {
        Self {
            valor: bandeja.valor,
            nome: bandeja.nome,
        }
    }
}

impl From<Resolucao> for ResolucaoDaImpressora {
    fn from(resolucao: Resolucao) -> Self {
        Self {
            x: resolucao.x,
            y: resolucao.y,
        }
    }
}

impl EscolhasDaImpressora {
    fn para_o_driver(&self) -> Escolhas {
        Escolhas {
            bandeja: self.bandeja,
            resolucao: self.resolucao.map(|resolucao| (resolucao.x, resolucao.y)),
            cor: self.cor.map(Cor::from),
        }
    }
}

impl From<OpcoesDaFila> for OpcoesDaImpressora {
    fn from(opcoes: OpcoesDaFila) -> Self {
        Self {
            bandejas: opcoes.bandejas.into_iter().map(Into::into).collect(),
            resolucoes: opcoes.resolucoes.into_iter().map(Into::into).collect(),
            cor: opcoes.cor.map(Into::into),
            atual: EscolhasDaImpressora {
                bandeja: opcoes.atual.bandeja,
                resolucao: opcoes
                    .atual
                    .resolucao
                    .map(|(x, y)| ResolucaoDaImpressora { x, y }),
                cor: opcoes.atual.cor.map(Into::into),
            },
        }
    }
}

/// Fala com o driver da fila sobre as opções que ele declara. Nos testes, um falso responde.
trait OpcoesDoDriver {
    fn ler(&self, fila: &str, salvo: Option<&[u8]>) -> Result<OpcoesDaFila, ErroPreferencias>;
    fn aplicar(
        &self,
        fila: &str,
        salvo: Option<&[u8]>,
        escolhas: &Escolhas,
    ) -> Result<Vec<u8>, ErroPreferencias>;
}

struct OpcoesDoSistema;

impl OpcoesDoDriver for OpcoesDoSistema {
    fn ler(&self, fila: &str, salvo: Option<&[u8]>) -> Result<OpcoesDaFila, ErroPreferencias> {
        plots_windows::opcoes_da_fila(fila, salvo)
    }

    fn aplicar(
        &self,
        fila: &str,
        salvo: Option<&[u8]>,
        escolhas: &Escolhas,
    ) -> Result<Vec<u8>, ErroPreferencias> {
        plots_windows::aplicar_opcoes_da_fila(fila, salvo, escolhas)
    }
}

/// O driver não respondeu ou não aceitou as opções: a fila sumida tem o código dela.
fn falha_do_driver(erro: ErroPreferencias, fila: &str) -> FalhaPreferencias {
    match erro {
        ErroPreferencias::FilaInexistente => FalhaPreferencias::nova(CodigoFalha::FilaSumiu, fila),
        ErroPreferencias::Envio(_) => FalhaPreferencias::nova(CodigoFalha::OpcaoRecusada, fila),
    }
}

/// A janela de preferencias do driver. Nos testes, um falso responde no lugar dela.
pub trait JanelaDoDriver {
    fn abrir(&self, fila: &str, salvo: Option<&[u8]>)
        -> Result<RespostaDaJanela, ErroPreferencias>;
    /// A janela desta fila ja esta aberta: traz para a frente em vez de abrir outra.
    fn trazer_para_frente(&self, fila: &str);
}

/// Janela real, dona da janela principal.
struct JanelaDoSistema {
    dono: isize,
}

impl JanelaDoDriver for JanelaDoSistema {
    fn abrir(
        &self,
        fila: &str,
        salvo: Option<&[u8]>,
    ) -> Result<RespostaDaJanela, ErroPreferencias> {
        plots_windows::abrir_janela_do_driver(fila, self.dono, salvo)
    }

    fn trazer_para_frente(&self, fila: &str) {
        plots_windows::trazer_janela_do_driver_para_frente(fila);
    }
}

/// Reserva a janela da fila. A reserva sai ao fim do escopo, mesmo em erro.
struct Reserva<'a> {
    abertas: &'a Mutex<BTreeSet<String>>,
    fila: String,
}

impl<'a> Reserva<'a> {
    fn nova(abertas: &'a Mutex<BTreeSet<String>>, fila: &str) -> Option<Self> {
        let novo = abertas
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(fila.to_owned());
        novo.then(|| Self {
            abertas,
            fila: fila.to_owned(),
        })
    }
}

impl Drop for Reserva<'_> {
    fn drop(&mut self) {
        self.abertas
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.fila);
    }
}

fn caminho_do_sistema() -> Result<PathBuf, ErroPerfis> {
    match std::env::var_os(VARIAVEL_PERFIS) {
        Some(valor) if !valor.is_empty() => Ok(PathBuf::from(valor)),
        _ => plots_perfis::caminho_padrao(),
    }
}

/// Le, muda e grava de uma vez. Se qualquer passo falhar, o arquivo fica como estava.
fn atualizar<T>(
    caminho: &std::path::Path,
    mudar: impl FnOnce(&mut Perfis) -> T,
) -> Result<T, ErroPerfis> {
    let _vez = GRAVACAO_DE_PERFIS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let mut perfis = plots_perfis::carregar(caminho)?;
    let resultado = mudar(&mut perfis);
    plots_perfis::salvar(caminho, &perfis)?;
    Ok(resultado)
}

/// Grava o ajuste aceito pela pessoa ou pelo driver. O arquivo inteiro volta como estava se
/// qualquer passo falhar.
fn gravar_ajuste(
    caminho: &std::path::Path,
    fila: &str,
    bytes: Vec<u8>,
) -> Result<(), FalhaPreferencias> {
    let gravado = atualizar(caminho, |perfis| {
        perfis.definir_ajuste_do_driver(fila, bytes)
    })
    .map_err(|erro| match erro {
        ErroPerfis::ConteudoGrande => FalhaPreferencias::nova(CodigoFalha::Grande, fila),
        _ => FalhaPreferencias::nova(CodigoFalha::NaoGravou, fila),
    })?;
    gravado.map_err(|recusa| match recusa {
        AjusteRecusado::Grande => FalhaPreferencias::nova(CodigoFalha::Grande, fila),
        AjusteRecusado::Vazio => FalhaPreferencias::nova(CodigoFalha::NaoAbriu, fila),
    })
}

/// Abre a janela e grava so se a pessoa aceitar. Uma janela por vez, por fila.
fn conduzir(
    janela: &impl JanelaDoDriver,
    abertas: &Mutex<BTreeSet<String>>,
    caminho: Result<PathBuf, ErroPerfis>,
    fila: &str,
) -> Result<FimDaJanela, FalhaPreferencias> {
    let Some(_reserva) = Reserva::nova(abertas, fila) else {
        janela.trazer_para_frente(fila);
        return Ok(FimDaJanela::JaAberta);
    };
    let nao_gravou = |_| FalhaPreferencias::nova(CodigoFalha::NaoGravou, fila);
    let caminho = caminho.map_err(nao_gravou)?;
    // Com o arquivo ilegivel, o que a pessoa escolhesse nao teria onde ficar: nem abre.
    let perfis = plots_perfis::carregar(&caminho).map_err(nao_gravou)?;
    let resposta =
        janela
            .abrir(fila, perfis.ajuste_do_driver(fila))
            .map_err(|erro| match erro {
                ErroPreferencias::FilaInexistente => {
                    FalhaPreferencias::nova(CodigoFalha::FilaSumiu, fila)
                }
                ErroPreferencias::Envio(_) => FalhaPreferencias::nova(CodigoFalha::NaoAbriu, fila),
            })?;
    match resposta {
        RespostaDaJanela::Cancelada => Ok(FimDaJanela::Cancelado),
        RespostaDaJanela::Aceita(bytes) => {
            gravar_ajuste(&caminho, fila, bytes)?;
            Ok(FimDaJanela::Salvo)
        }
    }
}

fn ler_estado(caminho: Result<PathBuf, ErroPerfis>) -> EstadoDasPreferencias {
    // Quem nao consegue ler os perfis ja ve o aviso da lista de impressoras.
    let Ok(perfis) = caminho.and_then(|caminho| plots_perfis::carregar(&caminho)) else {
        return EstadoDasPreferencias::default();
    };
    let salvas: Vec<String> = perfis.filas_com_ajuste().map(str::to_owned).collect();
    let duas_faces = salvas
        .iter()
        .filter(|fila| {
            perfis
                .ajuste_do_driver(fila)
                .is_some_and(plots_windows::ajuste_pede_duas_faces)
        })
        .cloned()
        .collect();
    EstadoDasPreferencias {
        salvas,
        ignoradas: perfis.ajustes_ignorados().to_vec(),
        duas_faces,
    }
}

fn voltar_ao_padrao_em(
    caminho: Result<PathBuf, ErroPerfis>,
    fila: &str,
) -> Result<(), FalhaPreferencias> {
    let nao_gravou = |_| FalhaPreferencias::nova(CodigoFalha::NaoGravou, fila);
    let caminho = caminho.map_err(nao_gravou)?;
    atualizar(&caminho, |perfis| {
        perfis.remover_ajuste_do_driver(fila);
    })
    .map_err(nao_gravou)
}

/// Lê do driver o que ele declara e parte do ajuste salvo para mostrar os valores atuais.
fn ler_opcoes_em(
    driver: &impl OpcoesDoDriver,
    caminho: Result<PathBuf, ErroPerfis>,
    fila: &str,
) -> Result<OpcoesDaImpressora, FalhaPreferencias> {
    let nao_abriu = |_| FalhaPreferencias::nova(CodigoFalha::NaoAbriu, fila);
    let caminho = caminho.map_err(nao_abriu)?;
    let perfis = plots_perfis::carregar(&caminho).map_err(nao_abriu)?;
    driver
        .ler(fila, perfis.ajuste_do_driver(fila))
        .map(OpcoesDaImpressora::from)
        .map_err(|erro| falha_do_driver(erro, fila))
}

/// Aplica as escolhas no ajuste da fila e grava só depois de o driver aceitar.
fn aplicar_em(
    driver: &impl OpcoesDoDriver,
    caminho: Result<PathBuf, ErroPerfis>,
    fila: &str,
    escolhas: &EscolhasDaImpressora,
) -> Result<(), FalhaPreferencias> {
    let nao_gravou = |_| FalhaPreferencias::nova(CodigoFalha::NaoGravou, fila);
    let caminho = caminho.map_err(nao_gravou)?;
    let perfis = plots_perfis::carregar(&caminho).map_err(nao_gravou)?;
    let bytes = driver
        .aplicar(
            fila,
            perfis.ajuste_do_driver(fila),
            &escolhas.para_o_driver(),
        )
        .map_err(|erro| falha_do_driver(erro, fila))?;
    gravar_ajuste(&caminho, fila, bytes)
}

/// Tira do arquivo o ajuste recusado, salvo se a pessoa ja gravou outro nesse meio tempo.
fn descartar_recusado(caminho: Result<PathBuf, ErroPerfis>, fila: &str, recusado: &[u8]) {
    let Ok(caminho) = caminho else { return };
    // Se nao der para gravar, o proximo envio recusa de novo e avisa de novo.
    let _ = atualizar(&caminho, |perfis| {
        if perfis.ajuste_do_driver(fila) == Some(recusado) {
            perfis.remover_ajuste_do_driver(fila);
        }
    });
}

/// De onde o envio tira o ajuste: o arquivo de perfis, lido a cada trabalho preparado.
struct FonteDoApp<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> FonteDeAjustes for FonteDoApp<R> {
    fn ajuste(&self, fila: &str) -> Option<Vec<u8>> {
        let caminho = caminho_do_sistema().ok()?;
        let perfis = plots_perfis::carregar(&caminho).ok()?;
        perfis.ajuste_do_driver(fila).map(<[u8]>::to_vec)
    }

    fn recusado(&self, fila: &str, ajuste: &[u8]) {
        marcar_recusa(fila);
        descartar_recusado(caminho_do_sistema(), fila, ajuste);
        // Sem interface para ouvir nao ha a quem avisar; o envio segue com o padrao da fila.
        let _ = self.app.emit(EVENTO_DE_RECUSA, fila);
    }
}

pub(crate) fn marcar_recusa(fila: &str) {
    RECUSAS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(fila.to_owned());
}

/// O driver recusou o ajuste desta fila desde a ultima pergunta. Conta uma vez so.
pub fn tomar_recusa(fila: &str) -> bool {
    RECUSAS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(fila)
}

/// Liga o envio ao arquivo de perfis. Chamado uma vez, ao iniciar.
pub fn ligar_ao_envio<R: Runtime>(app: &AppHandle<R>) {
    plots_windows::ligar_fonte_de_ajustes(Box::new(FonteDoApp { app: app.clone() }));
}

#[cfg(windows)]
fn janela_dona<R: Runtime>(app: &AppHandle<R>) -> isize {
    // Sem janela principal, o driver abre a dele sem dona.
    app.get_webview_window("main")
        .and_then(|janela| janela.hwnd().ok())
        .map_or(0, |identificador| identificador.0 as isize)
}

#[cfg(not(windows))]
fn janela_dona<R: Runtime>(_app: &AppHandle<R>) -> isize {
    0
}

/// Roda a conversa com a janela em uma thread com nome, so dela. O resultado volta pelo canal; uma
/// thread que nao inicia devolve `None`.
fn em_thread_propria(
    conversa: impl FnOnce() -> Result<FimDaJanela, FalhaPreferencias> + Send + 'static,
) -> Option<Receiver<Result<FimDaJanela, FalhaPreferencias>>> {
    let (envia, recebe) = tauri::async_runtime::channel(1);
    std::thread::Builder::new()
        .name("janela-do-driver".into())
        .spawn(move || {
            // Quem esperava pode ter ido embora: o resultado ja nao tem a quem servir.
            let _ = envia.blocking_send(conversa());
        })
        .ok()?;
    Some(recebe)
}

/// Thread que nao inicia, ou canal que cai antes da resposta, e janela que nao abriu.
async fn resposta_da_thread(
    recebe: Option<Receiver<Result<FimDaJanela, FalhaPreferencias>>>,
    fila: &str,
) -> Result<FimDaJanela, FalhaPreferencias> {
    let nao_abriu = || FalhaPreferencias::nova(CodigoFalha::NaoAbriu, fila);
    let Some(mut recebe) = recebe else {
        return Err(nao_abriu());
    };
    recebe.recv().await.unwrap_or_else(|| Err(nao_abriu()))
}

/// Abre a janela do driver. Roda fora da thread da interface, que continua respondendo.
#[tauri::command]
pub async fn abrir_preferencias<R: Runtime>(
    app: AppHandle<R>,
    fila: String,
) -> Result<FimDaJanela, FalhaPreferencias> {
    let dono = janela_dona(&app);
    let da_thread = fila.clone();
    let recebe = em_thread_propria(move || {
        conduzir(
            &JanelaDoSistema { dono },
            &ABERTAS,
            caminho_do_sistema(),
            &da_thread,
        )
    });
    resposta_da_thread(recebe, &fila).await
}

/// Em que pe esta o fechamento que espera a janela do driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fecho {
    Livre,
    Fechando,
    /// O prazo acabou com a janela ainda aberta: o proximo pedido segue sem ela.
    Desistiu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecisaoDoFecho {
    Deixar,
    Adiar { iniciar: bool },
}

fn decidir_fecho(janela_aberta: bool, fecho: &Mutex<Fecho>) -> DecisaoDoFecho {
    let mut fecho = fecho.lock().unwrap_or_else(PoisonError::into_inner);
    if !janela_aberta {
        *fecho = Fecho::Livre;
        return DecisaoDoFecho::Deixar;
    }
    match *fecho {
        Fecho::Livre => {
            *fecho = Fecho::Fechando;
            DecisaoDoFecho::Adiar { iniciar: true }
        }
        Fecho::Fechando => DecisaoDoFecho::Adiar { iniciar: false },
        Fecho::Desistiu => {
            *fecho = Fecho::Livre;
            DecisaoDoFecho::Deixar
        }
    }
}

fn definir_fecho(novo: Fecho) {
    *FECHO.lock().unwrap_or_else(PoisonError::into_inner) = novo;
}

/// Chamado no pedido de fechar, na thread da interface. Com a janela do driver aberta, pede que ela
/// feche e adia o pedido: nada aqui bloqueia, pois o diálogo precisa desta thread para devolver a
/// janela principal. Quando ela sair, ou o prazo acabar, o pedido volta sozinho.
pub fn adiar_fechamento<R: Runtime>(janela: &Window<R>) -> bool {
    match decidir_fecho(plots_windows::ha_janela_do_driver_aberta(), &FECHO) {
        DecisaoDoFecho::Deixar => false,
        DecisaoDoFecho::Adiar { iniciar: false } => true,
        DecisaoDoFecho::Adiar { iniciar: true } => {
            let pedida = janela.clone();
            let iniciada = std::thread::Builder::new()
                .name("fecha-janela-do-driver".into())
                .spawn(move || {
                    let fechou = plots_windows::fechar_janelas_do_driver(PRAZO_DA_JANELA);
                    definir_fecho(if fechou {
                        Fecho::Livre
                    } else {
                        Fecho::Desistiu
                    });
                    // Se o sistema recusar, o proximo pedido de fechamento tenta outra vez.
                    let _ = pedida.close();
                });
            if iniciada.is_err() {
                definir_fecho(Fecho::Livre);
            }
            iniciada.is_ok()
        }
    }
}

/// Fecha a janela do driver antes de destruir a principal, em uma thread que nao seja a da interface.
pub fn fechar_antes_de_destruir() {
    // Sem resposta no prazo, a janela do driver termina junto com o processo.
    let _ = plots_windows::fechar_janelas_do_driver(PRAZO_DA_JANELA);
}

/// Ultimo passo do programa. A janela principal ja nao existe, entao esperar aqui nao prende nada.
pub fn ao_encerrar() {
    // Sem resposta no prazo, a janela do driver termina junto com o processo.
    let _ = plots_windows::encerrar_janelas_do_driver(PRAZO_DA_JANELA);
}

#[tauri::command]
pub async fn estado_das_preferencias() -> EstadoDasPreferencias {
    tauri::async_runtime::spawn_blocking(|| ler_estado(caminho_do_sistema()))
        .await
        .unwrap_or_default()
}

/// Esquece o ajuste da fila: o proximo envio usa o padrao dela.
#[tauri::command]
pub async fn voltar_ao_padrao_da_fila(fila: String) -> Result<(), FalhaPreferencias> {
    let interrompida = FalhaPreferencias::nova(CodigoFalha::NaoGravou, &fila);
    tauri::async_runtime::spawn_blocking(move || voltar_ao_padrao_em(caminho_do_sistema(), &fila))
        .await
        .unwrap_or(Err(interrompida))
}

/// Le as opcoes que o driver da fila declara, sem abrir a janela dele.
#[tauri::command]
pub async fn opcoes_da_impressora(fila: String) -> Result<OpcoesDaImpressora, FalhaPreferencias> {
    let interrompida = FalhaPreferencias::nova(CodigoFalha::NaoAbriu, &fila);
    let da_thread = fila.clone();
    tauri::async_runtime::spawn_blocking(move || {
        ler_opcoes_em(&OpcoesDoSistema, caminho_do_sistema(), &da_thread)
    })
    .await
    .unwrap_or(Err(interrompida))
}

/// Aplica no ajuste salvo da fila as opcoes escolhidas no painel.
#[tauri::command]
pub async fn aplicar_opcoes_da_impressora(
    fila: String,
    escolhas: EscolhasDaImpressora,
) -> Result<(), FalhaPreferencias> {
    let interrompida = FalhaPreferencias::nova(CodigoFalha::NaoGravou, &fila);
    let da_thread = fila.clone();
    tauri::async_runtime::spawn_blocking(move || {
        aplicar_em(
            &OpcoesDoSistema,
            caminho_do_sistema(),
            &da_thread,
            &escolhas,
        )
    })
    .await
    .unwrap_or(Err(interrompida))
}

#[cfg(test)]
mod testes {
    use super::*;
    use plots_core::ModeloImpressora;
    use plots_windows::ErroEnvio;
    use std::cell::RefCell;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    static CONTADOR: AtomicU64 = AtomicU64::new(0);

    struct Pasta(PathBuf);

    impl Pasta {
        fn nova() -> Self {
            let sequencia = CONTADOR.fetch_add(1, Ordering::Relaxed);
            let caminho = std::env::temp_dir().join(format!(
                "plots-preferencias-{}-{sequencia}",
                std::process::id()
            ));
            fs::create_dir_all(&caminho).unwrap();
            Self(caminho)
        }

        fn arquivo(&self) -> PathBuf {
            self.0.join("perfis.toml")
        }
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Responde sempre o mesmo e guarda o ajuste com que foi aberta.
    struct Falsa {
        resposta: Result<RespostaDaJanela, ErroPreferencias>,
        recebido: RefCell<Vec<Option<Vec<u8>>>>,
        a_frente: RefCell<Vec<String>>,
    }

    impl Falsa {
        fn nova(resposta: Result<RespostaDaJanela, ErroPreferencias>) -> Self {
            Self {
                resposta,
                recebido: RefCell::new(Vec::new()),
                a_frente: RefCell::new(Vec::new()),
            }
        }
    }

    impl JanelaDoDriver for Falsa {
        fn abrir(
            &self,
            _fila: &str,
            salvo: Option<&[u8]>,
        ) -> Result<RespostaDaJanela, ErroPreferencias> {
            self.recebido.borrow_mut().push(salvo.map(<[u8]>::to_vec));
            self.resposta.clone()
        }

        fn trazer_para_frente(&self, fila: &str) {
            self.a_frente.borrow_mut().push(fila.to_owned());
        }
    }

    fn abrir(
        janela: &impl JanelaDoDriver,
        arquivo: &Path,
        fila: &str,
    ) -> Result<FimDaJanela, FalhaPreferencias> {
        let abertas = Mutex::new(BTreeSet::new());
        conduzir(janela, &abertas, Ok(arquivo.to_path_buf()), fila)
    }

    #[test]
    fn cancelar_nao_grava_nem_cria_o_arquivo() {
        let pasta = Pasta::nova();
        let janela = Falsa::nova(Ok(RespostaDaJanela::Cancelada));
        assert_eq!(
            abrir(&janela, &pasta.arquivo(), "Sala"),
            Ok(FimDaJanela::Cancelado)
        );
        assert!(!pasta.arquivo().exists());
    }

    #[test]
    fn cancelar_mantem_o_ajuste_que_ja_estava_salvo() {
        let pasta = Pasta::nova();
        let aceita = Falsa::nova(Ok(RespostaDaJanela::Aceita(vec![1, 2])));
        abrir(&aceita, &pasta.arquivo(), "Sala").unwrap();
        let antes = fs::read(pasta.arquivo()).unwrap();
        let cancela = Falsa::nova(Ok(RespostaDaJanela::Cancelada));
        abrir(&cancela, &pasta.arquivo(), "Sala").unwrap();
        assert_eq!(fs::read(pasta.arquivo()).unwrap(), antes);
    }

    #[test]
    fn aceitar_grava_e_a_proxima_janela_parte_do_salvo() {
        let pasta = Pasta::nova();
        let janela = Falsa::nova(Ok(RespostaDaJanela::Aceita(vec![1, 2, 3])));
        assert_eq!(
            abrir(&janela, &pasta.arquivo(), "Sala"),
            Ok(FimDaJanela::Salvo)
        );
        abrir(&janela, &pasta.arquivo(), "Sala").unwrap();
        assert_eq!(
            *janela.recebido.borrow(),
            [None, Some(vec![1u8, 2, 3])],
            "a primeira abre no padrao e a segunda no salvo"
        );
        assert_eq!(ler_estado(Ok(pasta.arquivo())).salvas, ["Sala".to_owned()]);
    }

    #[test]
    fn aceitar_nao_perde_os_perfis_nem_o_ajuste_de_outra_fila() {
        let pasta = Pasta::nova();
        let mut perfis = Perfis::new();
        perfis.definir_modelo("Sala", ModeloImpressora::Laser);
        perfis.definir_ajuste_do_driver("Outra", vec![9]).unwrap();
        plots_perfis::salvar(&pasta.arquivo(), &perfis).unwrap();
        let janela = Falsa::nova(Ok(RespostaDaJanela::Aceita(vec![1])));
        abrir(&janela, &pasta.arquivo(), "Sala").unwrap();
        let lidos = plots_perfis::carregar(&pasta.arquivo()).unwrap();
        assert_eq!(
            lidos.obter("Sala").and_then(|entrada| entrada.modelo),
            Some(ModeloImpressora::Laser)
        );
        assert_eq!(lidos.ajuste_do_driver("Outra"), Some(&[9u8][..]));
        assert_eq!(lidos.ajuste_do_driver("Sala"), Some(&[1u8][..]));
    }

    #[test]
    fn fila_que_sumiu_vira_mensagem_e_nao_grava() {
        let pasta = Pasta::nova();
        let janela = Falsa::nova(Err(ErroPreferencias::FilaInexistente));
        assert_eq!(
            abrir(&janela, &pasta.arquivo(), "Sala"),
            Err(FalhaPreferencias::nova(CodigoFalha::FilaSumiu, "Sala"))
        );
        assert!(!pasta.arquivo().exists());
    }

    #[test]
    fn driver_que_nao_abre_vira_mensagem() {
        let pasta = Pasta::nova();
        let janela = Falsa::nova(Err(ErroPreferencias::Envio(ErroEnvio::SemTrabalho)));
        assert_eq!(
            abrir(&janela, &pasta.arquivo(), "Sala"),
            Err(FalhaPreferencias::nova(CodigoFalha::NaoAbriu, "Sala"))
        );
    }

    #[test]
    fn arquivo_de_perfis_ilegivel_nao_abre_a_janela_nem_e_apagado() {
        let pasta = Pasta::nova();
        fs::write(pasta.arquivo(), "isto nao e toml = = =").unwrap();
        let janela = Falsa::nova(Ok(RespostaDaJanela::Aceita(vec![1])));
        assert_eq!(
            abrir(&janela, &pasta.arquivo(), "Sala"),
            Err(FalhaPreferencias::nova(CodigoFalha::NaoGravou, "Sala"))
        );
        assert!(janela.recebido.borrow().is_empty());
        assert_eq!(
            fs::read_to_string(pasta.arquivo()).unwrap(),
            "isto nao e toml = = ="
        );
    }

    #[test]
    fn ajuste_grande_demais_vira_mensagem_e_o_anterior_fica() {
        let pasta = Pasta::nova();
        let pequena = Falsa::nova(Ok(RespostaDaJanela::Aceita(vec![1])));
        abrir(&pequena, &pasta.arquivo(), "Sala").unwrap();
        let grande = vec![1u8; plots_perfis::LIMITE_AJUSTE_BYTES + 1];
        let janela = Falsa::nova(Ok(RespostaDaJanela::Aceita(grande)));
        assert_eq!(
            abrir(&janela, &pasta.arquivo(), "Sala"),
            Err(FalhaPreferencias::nova(CodigoFalha::Grande, "Sala"))
        );
        let lidos = plots_perfis::carregar(&pasta.arquivo()).unwrap();
        assert_eq!(lidos.ajuste_do_driver("Sala"), Some(&[1u8][..]));
    }

    /// Pede a mesma fila de novo enquanto a janela esta aberta.
    struct Reentrante<'a> {
        abertas: &'a Mutex<BTreeSet<String>>,
        arquivo: PathBuf,
        segunda: RefCell<Option<Result<FimDaJanela, FalhaPreferencias>>>,
        outra_fila: RefCell<Option<Result<FimDaJanela, FalhaPreferencias>>>,
        a_frente: RefCell<Vec<String>>,
    }

    impl JanelaDoDriver for Reentrante<'_> {
        fn abrir(
            &self,
            fila: &str,
            _salvo: Option<&[u8]>,
        ) -> Result<RespostaDaJanela, ErroPreferencias> {
            if fila == "Sala" {
                let simples = Falsa::nova(Ok(RespostaDaJanela::Cancelada));
                let de_novo = conduzir(&simples, self.abertas, Ok(self.arquivo.clone()), "Sala");
                *self.segunda.borrow_mut() = Some(de_novo);
                *self.a_frente.borrow_mut() = simples.a_frente.borrow().clone();
                let outra = conduzir(&simples, self.abertas, Ok(self.arquivo.clone()), "Outra");
                *self.outra_fila.borrow_mut() = Some(outra);
            }
            Ok(RespostaDaJanela::Cancelada)
        }

        fn trazer_para_frente(&self, _fila: &str) {}
    }

    #[test]
    fn outro_clique_com_a_janela_aberta_nao_abre_outra_e_a_vez_volta_ao_fechar() {
        let pasta = Pasta::nova();
        let abertas = Mutex::new(BTreeSet::new());
        let janela = Reentrante {
            abertas: &abertas,
            arquivo: pasta.arquivo(),
            segunda: RefCell::new(None),
            outra_fila: RefCell::new(None),
            a_frente: RefCell::new(Vec::new()),
        };
        assert_eq!(
            conduzir(&janela, &abertas, Ok(pasta.arquivo()), "Sala"),
            Ok(FimDaJanela::Cancelado)
        );
        assert_eq!(*janela.segunda.borrow(), Some(Ok(FimDaJanela::JaAberta)));
        assert_eq!(
            *janela.a_frente.borrow(),
            ["Sala".to_owned()],
            "a janela aberta vem para a frente"
        );
        assert_eq!(
            *janela.outra_fila.borrow(),
            Some(Ok(FimDaJanela::Cancelado)),
            "outra fila tem a sua propria janela"
        );
        assert!(abertas.lock().unwrap().is_empty());
    }

    #[test]
    fn a_vez_volta_mesmo_depois_de_erro() {
        let pasta = Pasta::nova();
        let abertas = Mutex::new(BTreeSet::new());
        let falha = Falsa::nova(Err(ErroPreferencias::FilaInexistente));
        assert!(conduzir(&falha, &abertas, Ok(pasta.arquivo()), "Sala").is_err());
        assert!(abertas.lock().unwrap().is_empty());
    }

    #[test]
    fn voltar_ao_padrao_tira_so_o_ajuste_e_e_seguro_repetir() {
        let pasta = Pasta::nova();
        let mut perfis = Perfis::new();
        perfis.definir_modelo("Sala", ModeloImpressora::Laser);
        perfis.definir_ajuste_do_driver("Sala", vec![1]).unwrap();
        plots_perfis::salvar(&pasta.arquivo(), &perfis).unwrap();
        voltar_ao_padrao_em(Ok(pasta.arquivo()), "Sala").unwrap();
        voltar_ao_padrao_em(Ok(pasta.arquivo()), "Sala").unwrap();
        let lidos = plots_perfis::carregar(&pasta.arquivo()).unwrap();
        assert_eq!(lidos.ajuste_do_driver("Sala"), None);
        assert!(lidos.obter("Sala").is_some());
    }

    #[test]
    fn ajuste_recusado_sai_do_arquivo_mas_um_novo_gravado_depois_fica() {
        let pasta = Pasta::nova();
        let mut perfis = Perfis::new();
        perfis.definir_ajuste_do_driver("Sala", vec![1]).unwrap();
        plots_perfis::salvar(&pasta.arquivo(), &perfis).unwrap();
        descartar_recusado(Ok(pasta.arquivo()), "Sala", &[2]);
        assert_eq!(
            plots_perfis::carregar(&pasta.arquivo())
                .unwrap()
                .ajuste_do_driver("Sala"),
            Some(&[1u8][..])
        );
        descartar_recusado(Ok(pasta.arquivo()), "Sala", &[1]);
        assert_eq!(
            plots_perfis::carregar(&pasta.arquivo())
                .unwrap()
                .ajuste_do_driver("Sala"),
            None
        );
    }

    #[test]
    fn estado_diz_quem_tem_ajuste_e_qual_estava_corrompido() {
        let pasta = Pasta::nova();
        fs::write(
            pasta.arquivo(),
            "versao = 1\n\n[ajustes_driver]\nBoa = \"AQID\"\nRuim = 12\n",
        )
        .unwrap();
        let estado = ler_estado(Ok(pasta.arquivo()));
        assert_eq!(estado.salvas, ["Boa".to_owned()]);
        assert_eq!(estado.ignoradas, ["Ruim".to_owned()]);
    }

    #[test]
    fn perfis_ilegiveis_nao_viram_lista_de_ajustes() {
        let pasta = Pasta::nova();
        fs::write(pasta.arquivo(), "= =").unwrap();
        assert_eq!(
            ler_estado(Ok(pasta.arquivo())),
            EstadoDasPreferencias::default()
        );
    }

    #[test]
    fn modelo_e_ajuste_gravados_ao_mesmo_tempo_nao_perdem_um_ao_outro() {
        const VEZES: usize = 60;
        let pasta = Pasta::nova();
        let caminho = pasta.arquivo();
        let (para_modelos, para_ajustes) = (caminho.clone(), caminho.clone());
        let modelos = std::thread::spawn(move || {
            for vez in 0..VEZES {
                crate::impressoras::gravar_modelo(
                    Ok(para_modelos.clone()),
                    &format!("Modelo {vez}"),
                    "laser",
                )
                .unwrap();
            }
        });
        let ajustes = std::thread::spawn(move || {
            for vez in 0..VEZES {
                atualizar(&para_ajustes, |perfis| {
                    perfis.definir_ajuste_do_driver(&format!("Ajuste {vez}"), vec![1])
                })
                .unwrap()
                .unwrap();
            }
        });
        modelos.join().unwrap();
        ajustes.join().unwrap();
        let lidos = plots_perfis::carregar(&caminho).unwrap();
        for vez in 0..VEZES {
            assert!(
                lidos.obter(&format!("Modelo {vez}")).is_some(),
                "modelo {vez}"
            );
            assert!(
                lidos.ajuste_do_driver(&format!("Ajuste {vez}")).is_some(),
                "ajuste {vez}"
            );
        }
    }

    #[test]
    fn estado_diz_qual_ajuste_pede_duas_faces_no_campo_publico() {
        const DM_SIZE: usize = 68;
        const DM_DUPLEX: usize = 94;
        const TAMANHO: u16 = 220;
        let ajuste = |faces: u8| {
            let mut bytes = vec![0u8; usize::from(TAMANHO)];
            bytes[DM_SIZE..DM_SIZE + 2].copy_from_slice(&TAMANHO.to_ne_bytes());
            bytes[DM_DUPLEX] = faces;
            bytes
        };
        let pasta = Pasta::nova();
        let mut perfis = Perfis::new();
        perfis.definir_ajuste_do_driver("Duas", ajuste(2)).unwrap();
        perfis.definir_ajuste_do_driver("Uma", ajuste(1)).unwrap();
        plots_perfis::salvar(&pasta.arquivo(), &perfis).unwrap();
        let estado = ler_estado(Ok(pasta.arquivo()));
        assert_eq!(estado.salvas, ["Duas".to_owned(), "Uma".to_owned()]);
        #[cfg(windows)]
        assert_eq!(estado.duas_faces, ["Duas".to_owned()]);
    }

    #[test]
    fn recusa_marcada_conta_uma_vez_e_so_para_a_fila_dela() {
        marcar_recusa("Fila da recusa");
        assert!(!tomar_recusa("Outra fila da recusa"));
        assert!(tomar_recusa("Fila da recusa"));
        assert!(!tomar_recusa("Fila da recusa"));
    }

    #[test]
    fn a_resposta_da_thread_chega_pelo_canal() {
        let recebe = em_thread_propria(|| Ok(FimDaJanela::Cancelado));
        let fim = tauri::async_runtime::block_on(resposta_da_thread(recebe, "Sala"));
        assert_eq!(fim, Ok(FimDaJanela::Cancelado));
    }

    #[test]
    fn thread_que_nao_inicia_vira_janela_que_nao_abriu() {
        let fim = tauri::async_runtime::block_on(resposta_da_thread(None, "Sala"));
        assert_eq!(
            fim,
            Err(FalhaPreferencias::nova(CodigoFalha::NaoAbriu, "Sala"))
        );
    }

    #[test]
    fn canal_que_cai_vira_janela_que_nao_abriu() {
        let (envia, recebe) = tauri::async_runtime::channel(1);
        drop(envia);
        let fim = tauri::async_runtime::block_on(resposta_da_thread(Some(recebe), "Sala"));
        assert_eq!(
            fim,
            Err(FalhaPreferencias::nova(CodigoFalha::NaoAbriu, "Sala"))
        );
    }

    #[test]
    fn fechar_sem_janela_aberta_segue_na_hora() {
        let fecho = Mutex::new(Fecho::Desistiu);
        assert_eq!(decidir_fecho(false, &fecho), DecisaoDoFecho::Deixar);
        assert_eq!(*fecho.lock().unwrap(), Fecho::Livre);
    }

    #[test]
    fn fechar_com_janela_aberta_adia_e_inicia_um_pedido_so() {
        let fecho = Mutex::new(Fecho::Livre);
        assert_eq!(
            decidir_fecho(true, &fecho),
            DecisaoDoFecho::Adiar { iniciar: true }
        );
        assert_eq!(
            decidir_fecho(true, &fecho),
            DecisaoDoFecho::Adiar { iniciar: false }
        );
    }

    #[test]
    fn janela_que_nao_fecha_no_prazo_nao_prende_o_programa() {
        let fecho = Mutex::new(Fecho::Desistiu);
        assert_eq!(decidir_fecho(true, &fecho), DecisaoDoFecho::Deixar);
    }

    /// Prova real pelo mesmo caminho do comando: a thread propria, a janela da fila de PDF e o
    /// fechamento por mensagem. Sem captura e sem teclas.
    #[cfg(windows)]
    #[test]
    #[ignore = "abre a janela real da fila de PDF; fecha sozinha, sem captura e sem teclas"]
    fn janela_real_abre_em_thread_propria_fecha_e_nao_grava_nada() {
        use std::time::Instant;
        const FILA: &str = "Microsoft Print to PDF";
        let pasta = Pasta::nova();
        let arquivo = pasta.arquivo();
        let caminho = arquivo.clone();
        let recebe = em_thread_propria(move || {
            conduzir(&JanelaDoSistema { dono: 0 }, &ABERTAS, Ok(caminho), FILA)
        });
        let mut recebe = recebe.expect("a thread inicia");
        let inicio = Instant::now();
        let mut titulos = Vec::new();
        while titulos.is_empty() && inicio.elapsed() < Duration::from_secs(30) {
            titulos = plots_windows::trazer_janela_do_driver_para_frente(FILA);
            std::thread::sleep(Duration::from_millis(100));
        }
        eprintln!("titulos da janela: {titulos:?}");
        assert!(!titulos.is_empty(), "a janela apareceu");
        assert!(plots_windows::ha_janela_do_driver_aberta());
        let fechou = plots_windows::fechar_janelas_do_driver(Duration::from_secs(10));
        assert!(fechou, "a janela fechou com a mensagem");
        let prazo = Instant::now();
        let fim = loop {
            match recebe.try_recv() {
                Ok(fim) => break fim,
                Err(_) if prazo.elapsed() < Duration::from_secs(10) => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(erro) => panic!("a thread nao respondeu: {erro}"),
            }
        };
        assert_eq!(fim, Ok(FimDaJanela::Cancelado));
        assert!(!arquivo.exists(), "nada foi gravado");
        assert!(!plots_windows::ha_janela_do_driver_aberta());
    }

    struct DriverDeOpcoes {
        leitura: Result<OpcoesDaFila, ErroPreferencias>,
        escrita: Result<Vec<u8>, ErroPreferencias>,
        salvo_lido: RefCell<Vec<Option<Vec<u8>>>>,
        escolhas_recebidas: RefCell<Option<Escolhas>>,
    }

    impl DriverDeOpcoes {
        fn novo(
            leitura: Result<OpcoesDaFila, ErroPreferencias>,
            escrita: Result<Vec<u8>, ErroPreferencias>,
        ) -> Self {
            Self {
                leitura,
                escrita,
                salvo_lido: RefCell::new(Vec::new()),
                escolhas_recebidas: RefCell::new(None),
            }
        }
    }

    impl OpcoesDoDriver for DriverDeOpcoes {
        fn ler(&self, _fila: &str, salvo: Option<&[u8]>) -> Result<OpcoesDaFila, ErroPreferencias> {
            self.salvo_lido.borrow_mut().push(salvo.map(<[u8]>::to_vec));
            self.leitura.clone()
        }

        fn aplicar(
            &self,
            _fila: &str,
            salvo: Option<&[u8]>,
            escolhas: &Escolhas,
        ) -> Result<Vec<u8>, ErroPreferencias> {
            self.salvo_lido.borrow_mut().push(salvo.map(<[u8]>::to_vec));
            *self.escolhas_recebidas.borrow_mut() = Some(escolhas.clone());
            self.escrita.clone()
        }
    }

    fn opcoes_de_exemplo() -> OpcoesDaFila {
        OpcoesDaFila {
            bandejas: vec![Bandeja {
                valor: 1,
                nome: "Bandeja 1".into(),
            }],
            resolucoes: vec![Resolucao { x: 600, y: 600 }],
            cor: Some(Cor::Colorida),
            atual: Escolhas {
                bandeja: Some(1),
                resolucao: Some((600, 600)),
                cor: Some(Cor::Colorida),
            },
        }
    }

    fn perfil_com_ajuste(fila: &str, ajuste: Vec<u8>) -> Pasta {
        let pasta = Pasta::nova();
        let mut perfis = Perfis::new();
        perfis.definir_modelo(fila, ModeloImpressora::Laser);
        perfis.definir_ajuste_do_driver(fila, ajuste).unwrap();
        plots_perfis::salvar(&pasta.arquivo(), &perfis).unwrap();
        pasta
    }

    #[test]
    fn ler_opcoes_parte_do_ajuste_salvo_e_traz_so_o_que_o_driver_declara() {
        let pasta = perfil_com_ajuste("Sala", vec![1, 2]);
        let driver = DriverDeOpcoes::novo(Ok(opcoes_de_exemplo()), Ok(Vec::new()));
        let opcoes = ler_opcoes_em(&driver, Ok(pasta.arquivo()), "Sala").unwrap();
        assert_eq!(*driver.salvo_lido.borrow(), [Some(vec![1u8, 2])]);
        assert_eq!(
            opcoes.bandejas,
            [BandejaDaImpressora {
                valor: 1,
                nome: "Bandeja 1".into()
            }]
        );
        assert_eq!(
            opcoes.resolucoes,
            [ResolucaoDaImpressora { x: 600, y: 600 }]
        );
        assert_eq!(opcoes.cor, Some(CorDaImpressora::Colorida));
        assert_eq!(
            opcoes.atual.resolucao,
            Some(ResolucaoDaImpressora { x: 600, y: 600 })
        );
    }

    #[test]
    fn aplicar_opcoes_grava_no_perfil_depois_de_o_driver_aceitar() {
        let pasta = perfil_com_ajuste("Sala", vec![1]);
        let driver = DriverDeOpcoes::novo(Ok(opcoes_de_exemplo()), Ok(vec![7, 7]));
        let escolhas = EscolhasDaImpressora {
            bandeja: Some(2),
            resolucao: Some(ResolucaoDaImpressora { x: 300, y: 300 }),
            cor: Some(CorDaImpressora::Monocromatica),
        };
        assert_eq!(
            aplicar_em(&driver, Ok(pasta.arquivo()), "Sala", &escolhas),
            Ok(())
        );
        let lidos = plots_perfis::carregar(&pasta.arquivo()).unwrap();
        assert_eq!(lidos.ajuste_do_driver("Sala"), Some(&[7u8, 7][..]));
        assert_eq!(
            *driver.escolhas_recebidas.borrow(),
            Some(Escolhas {
                bandeja: Some(2),
                resolucao: Some((300, 300)),
                cor: Some(Cor::Monocromatica),
            })
        );
    }

    #[test]
    fn driver_que_recusa_as_opcoes_nao_grava_e_o_ajuste_anterior_fica() {
        let pasta = perfil_com_ajuste("Sala", vec![1]);
        let driver = DriverDeOpcoes::novo(
            Ok(opcoes_de_exemplo()),
            Err(ErroPreferencias::Envio(ErroEnvio::Resposta("não aceitou"))),
        );
        let escolhas = EscolhasDaImpressora {
            bandeja: None,
            resolucao: Some(ResolucaoDaImpressora { x: 600, y: 600 }),
            cor: None,
        };
        assert_eq!(
            aplicar_em(&driver, Ok(pasta.arquivo()), "Sala", &escolhas),
            Err(FalhaPreferencias::nova(CodigoFalha::OpcaoRecusada, "Sala"))
        );
        let lidos = plots_perfis::carregar(&pasta.arquivo()).unwrap();
        assert_eq!(lidos.ajuste_do_driver("Sala"), Some(&[1u8][..]));
    }

    #[test]
    fn fila_que_sumiu_aparece_com_o_codigo_dela_ao_ler_e_ao_aplicar() {
        let pasta = Pasta::nova();
        let driver = DriverDeOpcoes::novo(
            Err(ErroPreferencias::FilaInexistente),
            Err(ErroPreferencias::FilaInexistente),
        );
        let escolhas = EscolhasDaImpressora {
            bandeja: None,
            resolucao: None,
            cor: None,
        };
        assert_eq!(
            ler_opcoes_em(&driver, Ok(pasta.arquivo()), "Sumida"),
            Err(FalhaPreferencias::nova(CodigoFalha::FilaSumiu, "Sumida"))
        );
        assert_eq!(
            aplicar_em(&driver, Ok(pasta.arquivo()), "Sumida", &escolhas),
            Err(FalhaPreferencias::nova(CodigoFalha::FilaSumiu, "Sumida"))
        );
        assert!(!pasta.arquivo().exists(), "nada foi gravado");
    }

    #[test]
    fn arquivo_de_perfis_ilegivel_nao_promete_opcoes_nem_grava() {
        let pasta = Pasta::nova();
        fs::write(pasta.arquivo(), "nao e toml = =").unwrap();
        let driver = DriverDeOpcoes::novo(Ok(opcoes_de_exemplo()), Ok(vec![7]));
        let escolhas = EscolhasDaImpressora {
            bandeja: None,
            resolucao: None,
            cor: None,
        };
        assert_eq!(
            ler_opcoes_em(&driver, Ok(pasta.arquivo()), "Sala"),
            Err(FalhaPreferencias::nova(CodigoFalha::NaoAbriu, "Sala"))
        );
        assert_eq!(
            aplicar_em(&driver, Ok(pasta.arquivo()), "Sala", &escolhas),
            Err(FalhaPreferencias::nova(CodigoFalha::NaoGravou, "Sala"))
        );
        assert!(driver.salvo_lido.borrow().is_empty());
        assert_eq!(
            fs::read_to_string(pasta.arquivo()).unwrap(),
            "nao e toml = ="
        );
    }
}
