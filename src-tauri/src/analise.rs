//! Análise de tinta do documento aberto.
//!
//! O comando só prepara o trabalho: a leitura do conteúdo, a interpretação da seleção e a
//! validação acontecem antes, e o desenho das páginas corre em uma thread própria. Cada página
//! vira uma cobertura guardada em memória; o resultado é montado a partir dela, sem repetir a
//! conta em outro lugar.

use crate::csv_tinta::{self, PaginaDeTinta, SelecaoDeTinta};
use crate::documento::ErroDocumento;
use crate::leitura_pdf;
use crate::mutool::{self, ErroMutool};
use crate::plotinfo;
use plots_core::{DimensoesPagina, LIMITE_COPIAS};
use plots_info::{
    cartucho, cobertura_de_bgra, estimativa_da_pagina, estimativa_da_selecao,
    estimativa_do_documento, pagina_mais_cara, selecao_de_texto, Cobertura, ConfiguracaoDeTinta,
    Consumo, Custo, EstadoDoCartucho, ModoDeCor, PerfilDeTinta, Qualidade, SelecaoDePaginas,
};
use plots_windows::analise::{ErroAnalise, PaginaRgb, SessaoDeAnalise};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use tauri::{AppHandle, Emitter, State};

/// Nome dos eventos que a interface escuta.
pub const EVENTO_PROGRESSO: &str = "plotinfo://progresso";
pub const EVENTO_PAGINA: &str = "plotinfo://pagina";
pub const EVENTO_FIM: &str = "plotinfo://fim";
pub const EVENTO_FALHA: &str = "plotinfo://falha";

/// Quantos documentos concluídos ainda aceitam recálculo pelo cache.
const MEMORIA_DE_ANALISES: usize = 32;

/// Quantos conteúdos diferentes o cache de cobertura guarda ao mesmo tempo.
const MEMORIA_DO_CACHE: usize = 8;

/// Código de aviso entregue quando o pedido era CMYK e a análise terminou no modo simples.
/// A interface traduz o código; o detalhe técnico vai em `aviso_detalhe`.
const AVISO_SEM_CONVERSOR: &str = "sem_conversor";
const AVISO_FALHA_DO_CONVERSOR: &str = "falha_do_conversor";
const AVISO_MODO_SIMPLES: &str = "modo_simples";

/// Cobertura de uma página, em percentual por canal.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoberturaDaPagina {
    pub c: f64,
    pub m: f64,
    pub y: f64,
    pub k: f64,
    pub total: f64,
}

impl From<Cobertura> for CoberturaDaPagina {
    fn from(cobertura: Cobertura) -> Self {
        Self {
            c: cobertura.c,
            m: cobertura.m,
            y: cobertura.y,
            k: cobertura.k,
            total: cobertura.total(),
        }
    }
}

/// Tinta em mililitros, por canal.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TintaCalculada {
    pub c: f64,
    pub m: f64,
    pub y: f64,
    pub k: f64,
    pub total: f64,
}

impl From<Consumo> for TintaCalculada {
    fn from(consumo: Consumo) -> Self {
        Self {
            c: consumo.c,
            m: consumo.m,
            y: consumo.y,
            k: consumo.k,
            total: consumo.total(),
        }
    }
}

/// Custo na moeda do perfil, por canal.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustoCalculado {
    pub c: f64,
    pub m: f64,
    pub y: f64,
    pub k: f64,
    pub total: f64,
}

impl From<Custo> for CustoCalculado {
    fn from(custo: Custo) -> Self {
        Self {
            c: custo.c,
            m: custo.m,
            y: custo.y,
            k: custo.k,
            total: custo.total(),
        }
    }
}

/// Situação do reservatório depois da seleção.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartuchoCalculado {
    pub percentual_consumido: f64,
    /// `None` quando a seleção não gasta aquele canal.
    pub copias_ate_esvaziar: Option<u64>,
    pub canal_que_acaba_primeiro: usize,
}

impl From<EstadoDoCartucho> for CartuchoCalculado {
    fn from(estado: EstadoDoCartucho) -> Self {
        Self {
            percentual_consumido: estado.percentual_consumido,
            copias_ate_esvaziar: (estado.copias_ate_esvaziar != u64::MAX)
                .then_some(estado.copias_ate_esvaziar),
            canal_que_acaba_primeiro: estado.canal_que_acaba_primeiro,
        }
    }
}

/// Totais das páginas escolhidas, já repetidos pelas cópias.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumoDaSelecao {
    pub paginas: Vec<usize>,
    pub quantidade: usize,
    pub total_com_copias: usize,
    pub tinta: TintaCalculada,
    pub custo: CustoCalculado,
    pub cartucho: CartuchoCalculado,
}

/// Página de maior custo no documento.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginaMaisCara {
    pub numero: usize,
    pub tinta: TintaCalculada,
    pub custo: CustoCalculado,
}

/// Totais do documento inteiro, em uma cópia.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumoDoDocumento {
    pub tinta: TintaCalculada,
    pub custo: CustoCalculado,
    pub cobertura_media: CoberturaDaPagina,
    pub ml_medio_por_pagina: f64,
    pub pagina_mais_cara: Option<PaginaMaisCara>,
}

/// De onde vieram os canais da análise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModoUsado {
    /// A fórmula simples sobre os pixels desenhados pela biblioteca.
    Simples,
    /// A separação feita pelo conversor externo, lida dos PAMs em CMYK.
    Cmyk,
}

/// O que a análise entrega ao fim.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoDaAnalise {
    pub total_de_paginas: usize,
    pub modo_usado: ModoUsado,
    /// Código do motivo de o modo simples ter sido usado, quando o pedido era CMYK.
    pub aviso: Option<String>,
    /// Detalhe técnico do aviso, para diagnóstico; não substitui a mensagem traduzida.
    pub aviso_detalhe: Option<String>,
    pub dpi: u32,
    pub perfil: String,
    pub moeda: String,
    pub qualidade: Qualidade,
    pub copias: usize,
    pub documento_inteiro: bool,
    pub selecao: ResumoDaSelecao,
    pub documento: ResumoDoDocumento,
}

/// Progresso do documento; a seleção chega preenchida assim que as páginas dela estiverem
/// prontas.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressoDaAnalise {
    pub id: String,
    pub feitas: usize,
    pub total: usize,
    pub selecao: Option<ResumoDaSelecao>,
}

/// Cobertura e estimativa de uma página recém-desenhada.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginaAnalisada {
    pub id: String,
    pub pagina: usize,
    pub largura_mm: f64,
    pub altura_mm: f64,
    pub cobertura: CoberturaDaPagina,
    pub tinta: TintaCalculada,
    pub custo: CustoCalculado,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FimDaAnalise {
    pub id: String,
    pub resultado: ResultadoDaAnalise,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FalhaDaAnalise {
    pub id: String,
    pub codigo: CodigoFalhaAnalise,
    pub detalhe: Option<String>,
}

/// Por que uma análise não foi concluída. A interface traduz o código.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodigoFalhaAnalise {
    Documento,
    ConfiguracaoInvalida,
    SelecaoInvalida,
    CopiasInvalidas,
    PerfilNaoEncontrado,
    Renderizacao,
    Biblioteca,
    Calculo,
    AnaliseDesconhecida,
    CacheAusente,
    AindaNaoAnalisada,
    Cancelado,
    Interrompido,
    IniciarTrabalho,
    Exportacao,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FalhaAnalise {
    pub codigo: CodigoFalhaAnalise,
    pub detalhe: Option<String>,
}

impl FalhaAnalise {
    fn novo(codigo: CodigoFalhaAnalise) -> Self {
        Self {
            codigo,
            detalhe: None,
        }
    }

    fn com_detalhe(codigo: CodigoFalhaAnalise, detalhe: impl ToString) -> Self {
        Self {
            codigo,
            detalhe: Some(detalhe.to_string()),
        }
    }

    fn da_renderizacao(erro: ErroAnalise) -> Self {
        match erro {
            ErroAnalise::Biblioteca(detalhe) => {
                Self::com_detalhe(CodigoFalhaAnalise::Biblioteca, detalhe)
            }
            outro => Self::com_detalhe(CodigoFalhaAnalise::Renderizacao, outro),
        }
    }
}

impl std::fmt::Display for FalhaAnalise {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.codigo)?;
        if let Some(detalhe) = &self.detalhe {
            write!(f, ": {detalhe}")?;
        }
        Ok(())
    }
}

impl std::error::Error for FalhaAnalise {}

impl From<ErroDocumento> for FalhaAnalise {
    fn from(erro: ErroDocumento) -> Self {
        Self::com_detalhe(
            CodigoFalhaAnalise::Documento,
            format!("{:?}: {}", erro.codigo, erro.nome),
        )
    }
}

fn validar_copias(copias: u32) -> Result<usize, FalhaAnalise> {
    let copias = usize::try_from(copias)
        .map_err(|_| FalhaAnalise::com_detalhe(CodigoFalhaAnalise::CopiasInvalidas, copias))?;
    if !(1..=LIMITE_COPIAS).contains(&copias) {
        return Err(FalhaAnalise::com_detalhe(
            CodigoFalhaAnalise::CopiasInvalidas,
            copias,
        ));
    }
    Ok(copias)
}

fn chave_de(impressao: &str, configuracao: &ConfiguracaoDeTinta) -> ChaveDeCache {
    ChaveDeCache {
        impressao: impressao.to_owned(),
        dpi: configuracao.dpi,
        modo: codigo_do_modo(configuracao.modo),
    }
}

fn codigo_do_modo(modo: ModoDeCor) -> u8 {
    match modo {
        ModoDeCor::Cmyk => 0,
        ModoDeCor::Simples => 1,
    }
}

fn modo_do_codigo(codigo: u8) -> ModoUsado {
    if codigo == codigo_do_modo(ModoDeCor::Cmyk) {
        ModoUsado::Cmyk
    } else {
        ModoUsado::Simples
    }
}

fn chave_do_modo(chave: &ChaveDeCache, modo: ModoDeCor) -> ChaveDeCache {
    ChaveDeCache {
        impressao: chave.impressao.clone(),
        dpi: chave.dpi,
        modo: codigo_do_modo(modo),
    }
}

/// O que a thread precisa para desenhar e montar o resultado.
#[derive(Debug)]
struct Preparo {
    chave: ChaveDeCache,
    bytes: Vec<u8>,
    dimensoes: Vec<DimensoesPagina>,
    selecao: SelecaoDePaginas,
    pagina_atual: Option<usize>,
    configuracao: ConfiguracaoDeTinta,
    perfil: PerfilDeTinta,
    copias: usize,
    /// Conteúdo já resolvido para leitura: o PDF original ou o derivado de uma imagem.
    pdf: PathBuf,
}

impl Preparo {
    fn total(&self) -> usize {
        self.dimensoes.len()
    }
}

fn preparar(
    caminho: &str,
    paginas: Option<&str>,
    copias: u32,
    configuracao: ConfiguracaoDeTinta,
    pagina_atual: Option<usize>,
) -> Result<Preparo, FalhaAnalise> {
    configuracao.validar().map_err(|erro| {
        FalhaAnalise::com_detalhe(CodigoFalhaAnalise::ConfiguracaoInvalida, erro)
    })?;
    let copias = validar_copias(copias)?;
    let (lido, bytes) = leitura_pdf::ler_pdf_com_bytes(caminho)?;
    let total = lido.dimensoes.len();
    if total == 0 {
        return Err(FalhaAnalise::novo(CodigoFalhaAnalise::Documento));
    }
    let selecao = selecao_de_texto(paginas.unwrap_or(""), total)
        .map_err(|erro| FalhaAnalise::com_detalhe(CodigoFalhaAnalise::SelecaoInvalida, erro))?;
    let perfil = plotinfo::perfil_guardado(&configuracao.perfil).ok_or_else(|| {
        FalhaAnalise::com_detalhe(
            CodigoFalhaAnalise::PerfilNaoEncontrado,
            configuracao.perfil.clone(),
        )
    })?;
    let pagina_atual = pagina_atual.filter(|numero| (1..=total).contains(numero));
    Ok(Preparo {
        chave: chave_de(&lido.impressao, &configuracao),
        bytes,
        dimensoes: lido.dimensoes,
        selecao,
        pagina_atual,
        configuracao,
        perfil,
        copias,
        pdf: PathBuf::from(lido.conteudo),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ChaveDeCache {
    impressao: String,
    dpi: u32,
    modo: u8,
}

struct EntradaDeCache {
    dimensoes: Vec<DimensoesPagina>,
    coberturas: Vec<Option<Cobertura>>,
    uso: u64,
}

#[derive(Clone)]
struct Concluida {
    chave: ChaveDeCache,
    selecao: SelecaoDePaginas,
    copias: usize,
    configuracao: ConfiguracaoDeTinta,
    perfil: PerfilDeTinta,
    uso: u64,
}

#[derive(Default)]
struct EstadoInterno {
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
    cache: Mutex<HashMap<ChaveDeCache, EntradaDeCache>>,
    concluidas: Mutex<HashMap<String, Concluida>>,
    sequencia: AtomicU64,
    uso: AtomicU64,
}

/// Estado da análise, compartilhado com cada trabalho.
#[derive(Clone, Default)]
pub struct Analises(Arc<EstadoInterno>);

impl Analises {
    fn proximo_uso(&self) -> u64 {
        self.0.uso.fetch_add(1, Ordering::Relaxed)
    }

    fn novo_id(&self) -> String {
        let sequencia = self.0.sequencia.fetch_add(1, Ordering::Relaxed);
        format!("analise-{}-{sequencia}", std::process::id())
    }

    fn registrar(&self, id: &str, cancelamento: Arc<AtomicBool>) {
        let mut jobs = self.0.jobs.lock().unwrap_or_else(PoisonError::into_inner);
        jobs.insert(id.to_owned(), cancelamento);
    }

    fn remover(&self, id: &str) {
        let mut jobs = self.0.jobs.lock().unwrap_or_else(PoisonError::into_inner);
        jobs.remove(id);
    }

    /// Pede o cancelamento do trabalho. Um identificador que já terminou não é erro: o pedido
    /// pode chegar depois do fim.
    pub fn cancelar(&self, id: &str) -> bool {
        let jobs = self.0.jobs.lock().unwrap_or_else(PoisonError::into_inner);
        jobs.get(id)
            .map(|cancelamento| {
                cancelamento.store(true, Ordering::Relaxed);
                true
            })
            .unwrap_or(false)
    }

    fn abrir_cache(
        &self,
        chave: &ChaveDeCache,
        dimensoes: &[DimensoesPagina],
    ) -> Result<Vec<Option<Cobertura>>, FalhaAnalise> {
        let uso = self.proximo_uso();
        let mut cache = self.0.cache.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(entrada) = cache.get_mut(chave) {
            if entrada.dimensoes == dimensoes {
                entrada.uso = uso;
                return Ok(entrada.coberturas.clone());
            }
            cache.remove(chave);
        }
        let mut coberturas = Vec::new();
        coberturas
            .try_reserve_exact(dimensoes.len())
            .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Interrompido))?;
        coberturas.resize(dimensoes.len(), None);
        cache.insert(
            chave.clone(),
            EntradaDeCache {
                dimensoes: dimensoes.to_vec(),
                coberturas: coberturas.clone(),
                uso,
            },
        );
        limitar_por_uso(&mut cache, MEMORIA_DO_CACHE, |entrada| entrada.uso);
        Ok(coberturas)
    }

    fn gravar_cobertura(
        &self,
        chave: &ChaveDeCache,
        dimensoes: &[DimensoesPagina],
        indice: usize,
        cobertura: Cobertura,
    ) {
        let uso = self.proximo_uso();
        let mut cache = self.0.cache.lock().unwrap_or_else(PoisonError::into_inner);
        if cache.get(chave).is_none() {
            let mut coberturas = Vec::new();
            if coberturas.try_reserve_exact(dimensoes.len()).is_err() {
                return;
            }
            coberturas.resize(dimensoes.len(), None);
            cache.insert(
                chave.clone(),
                EntradaDeCache {
                    dimensoes: dimensoes.to_vec(),
                    coberturas,
                    uso,
                },
            );
            limitar_por_uso(&mut cache, MEMORIA_DO_CACHE, |entrada| entrada.uso);
        }
        if let Some(entrada) = cache.get_mut(chave) {
            if entrada.dimensoes.len() != dimensoes.len() {
                return;
            }
            if let Some(slot) = entrada.coberturas.get_mut(indice) {
                *slot = Some(cobertura);
                entrada.uso = uso;
            }
        }
    }

    fn ler_cache(
        &self,
        chave: &ChaveDeCache,
    ) -> Option<(Vec<DimensoesPagina>, Vec<Option<Cobertura>>)> {
        let uso = self.proximo_uso();
        let mut cache = self.0.cache.lock().unwrap_or_else(PoisonError::into_inner);
        let entrada = cache.get_mut(chave)?;
        entrada.uso = uso;
        Some((entrada.dimensoes.clone(), entrada.coberturas.clone()))
    }

    fn guardar_concluida(&self, id: &str, chave: ChaveDeCache, preparo: &Preparo) {
        let uso = self.proximo_uso();
        let mut concluidas = self
            .0
            .concluidas
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        concluidas.insert(
            id.to_owned(),
            Concluida {
                chave,
                selecao: preparo.selecao.clone(),
                copias: preparo.copias,
                configuracao: preparo.configuracao.clone(),
                perfil: preparo.perfil.clone(),
                uso,
            },
        );
        limitar_por_uso(&mut concluidas, MEMORIA_DE_ANALISES, |entrada| entrada.uso);
    }

    fn chave_concluida(&self, id: &str) -> Option<ChaveDeCache> {
        self.retrato_concluida(id).map(|entrada| entrada.chave)
    }

    fn retrato_concluida(&self, id: &str) -> Option<Concluida> {
        let uso = self.proximo_uso();
        let mut concluidas = self
            .0
            .concluidas
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let entrada = concluidas.get_mut(id)?;
        entrada.uso = uso;
        Some(entrada.clone())
    }

    /// Guarda a seleção e a configuração da última conta concluída; a exportação usa esse retrato.
    fn atualizar_concluida(
        &self,
        id: &str,
        chave: ChaveDeCache,
        selecao: SelecaoDePaginas,
        copias: usize,
        configuracao: ConfiguracaoDeTinta,
        perfil: PerfilDeTinta,
    ) {
        let uso = self.proximo_uso();
        let mut concluidas = self
            .0
            .concluidas
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(entrada) = concluidas.get_mut(id) {
            entrada.chave = chave;
            entrada.selecao = selecao;
            entrada.copias = copias;
            entrada.configuracao = configuracao;
            entrada.perfil = perfil;
            entrada.uso = uso;
        }
    }

    /// Recalcula com o cache já pronto. O perfil vem de quem chama para o cálculo não depender
    /// do disco no meio da conta.
    fn recalcular_com_perfil(
        &self,
        id: &str,
        configuracao: &ConfiguracaoDeTinta,
        paginas: Option<&str>,
        copias: u32,
        perfil: &PerfilDeTinta,
    ) -> Result<ResultadoDaAnalise, FalhaAnalise> {
        configuracao.validar().map_err(|erro| {
            FalhaAnalise::com_detalhe(CodigoFalhaAnalise::ConfiguracaoInvalida, erro)
        })?;
        let copias = validar_copias(copias)?;
        let base = self
            .chave_concluida(id)
            .ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::AnaliseDesconhecida))?;
        let chave_pedida = ChaveDeCache {
            impressao: base.impressao.clone(),
            dpi: configuracao.dpi,
            modo: codigo_do_modo(configuracao.modo),
        };
        let achado = self.ler_cache(&chave_pedida);
        let (chave, dimensoes, coberturas) = match achado {
            Some((dimensoes, coberturas)) => {
                if dimensoes.is_empty() {
                    return Err(FalhaAnalise::novo(CodigoFalhaAnalise::Documento));
                }
                // Uma seleção inválida é recusada antes de a cobertura incompleta aparecer.
                selecao_de_texto(paginas.unwrap_or(""), dimensoes.len()).map_err(|erro| {
                    FalhaAnalise::com_detalhe(CodigoFalhaAnalise::SelecaoInvalida, erro)
                })?;
                if coberturas.iter().all(Option::is_some) {
                    (chave_pedida, dimensoes, coberturas)
                } else if configuracao.modo == ModoDeCor::Cmyk
                    && base.modo == codigo_do_modo(ModoDeCor::Simples)
                    && base.dpi == configuracao.dpi
                {
                    // A análise anterior caiu no modo simples e está guardada sob essa chave.
                    let (dimensoes, coberturas) = self.ler_cache(&base).ok_or_else(|| {
                        FalhaAnalise::com_detalhe(
                            CodigoFalhaAnalise::CacheAusente,
                            format!("{} dpi, modo simples", configuracao.dpi),
                        )
                    })?;
                    (base, dimensoes, coberturas)
                } else {
                    return Err(FalhaAnalise::novo(CodigoFalhaAnalise::AindaNaoAnalisada));
                }
            }
            None if configuracao.modo == ModoDeCor::Cmyk
                && base.modo == codigo_do_modo(ModoDeCor::Simples)
                && base.dpi == configuracao.dpi =>
            {
                let (dimensoes, coberturas) = self.ler_cache(&base).ok_or_else(|| {
                    FalhaAnalise::com_detalhe(
                        CodigoFalhaAnalise::CacheAusente,
                        format!("{} dpi, modo simples", configuracao.dpi),
                    )
                })?;
                (base, dimensoes, coberturas)
            }
            None => {
                return Err(FalhaAnalise::com_detalhe(
                    CodigoFalhaAnalise::CacheAusente,
                    format!(
                        "{} dpi, modo {}",
                        configuracao.dpi,
                        codigo_do_modo(configuracao.modo)
                    ),
                ))
            }
        };
        let modo_usado = modo_do_codigo(chave.modo);
        if dimensoes.is_empty() {
            return Err(FalhaAnalise::novo(CodigoFalhaAnalise::Documento));
        }
        let selecao = selecao_de_texto(paginas.unwrap_or(""), dimensoes.len())
            .map_err(|erro| FalhaAnalise::com_detalhe(CodigoFalhaAnalise::SelecaoInvalida, erro))?;
        let completas: Option<Vec<Cobertura>> = coberturas.iter().copied().collect();
        let completas =
            completas.ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::AindaNaoAnalisada))?;
        let mut resultado = montar_resultado(
            &completas,
            &dimensoes,
            &selecao,
            copias,
            configuracao,
            perfil,
            modo_usado,
        )?;
        if configuracao.modo == ModoDeCor::Cmyk && modo_usado == ModoUsado::Simples {
            resultado.aviso = Some(AVISO_MODO_SIMPLES.to_owned());
        }
        self.atualizar_concluida(
            id,
            chave,
            selecao,
            copias,
            configuracao.clone(),
            perfil.clone(),
        );
        Ok(resultado)
    }
}

fn limitar_por_uso<K, T>(mapa: &mut HashMap<K, T>, maximo: usize, uso: impl Fn(&T) -> u64)
where
    K: Clone + Eq + std::hash::Hash,
{
    if mapa.len() <= maximo {
        return;
    }
    while mapa.len() > maximo {
        let Some(chave) = mapa
            .iter()
            .min_by_key(|(_, entrada)| uso(entrada))
            .map(|(chave, _)| chave.clone())
        else {
            return;
        };
        mapa.remove(&chave);
    }
}

fn validar_copias_em_numero(copias: usize) -> Result<usize, FalhaAnalise> {
    if !(1..=LIMITE_COPIAS).contains(&copias) {
        return Err(FalhaAnalise::com_detalhe(
            CodigoFalhaAnalise::CopiasInvalidas,
            copias,
        ));
    }
    Ok(copias)
}

fn calcular_paginas(
    coberturas: &[Cobertura],
    dimensoes: &[DimensoesPagina],
    perfil: &PerfilDeTinta,
    qualidade: Qualidade,
) -> Result<(Vec<Consumo>, Vec<Custo>), FalhaAnalise> {
    let mut consumos = Vec::new();
    consumos
        .try_reserve_exact(coberturas.len())
        .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
    let mut custos = Vec::new();
    custos
        .try_reserve_exact(coberturas.len())
        .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
    for (indice, cobertura) in coberturas.iter().enumerate() {
        let dimensao = dimensoes
            .get(indice)
            .ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
        let estimativa = estimativa_da_pagina(*cobertura, *dimensao, perfil, qualidade)
            .map_err(|erro| FalhaAnalise::com_detalhe(CodigoFalhaAnalise::Calculo, erro))?;
        consumos.push(estimativa.tinta);
        custos.push(estimativa.custo);
    }
    Ok((consumos, custos))
}

fn media_de_cobertura(coberturas: &[Cobertura]) -> Cobertura {
    if coberturas.is_empty() {
        return Cobertura::default();
    }
    let mut soma = [0.0f64; 4];
    for cobertura in coberturas {
        for (acumulado, valor) in soma.iter_mut().zip(cobertura.valores()) {
            *acumulado += valor;
        }
    }
    let divisor = coberturas.len() as f64;
    Cobertura::new(
        soma[0] / divisor,
        soma[1] / divisor,
        soma[2] / divisor,
        soma[3] / divisor,
    )
}

fn resumo_da_selecao(
    coberturas_da_selecao: &[Cobertura],
    dimensoes: &[DimensoesPagina],
    selecao: &SelecaoDePaginas,
    copias: usize,
    perfil: &PerfilDeTinta,
    qualidade: Qualidade,
) -> Result<ResumoDaSelecao, FalhaAnalise> {
    if coberturas_da_selecao.len() != selecao.quantidade() {
        return Err(FalhaAnalise::novo(CodigoFalhaAnalise::Calculo));
    }
    let mut consumos = Vec::new();
    consumos
        .try_reserve_exact(coberturas_da_selecao.len())
        .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
    for (posicao, numero) in selecao.numeros().iter().enumerate() {
        let indice = numero
            .checked_sub(1)
            .ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
        let dimensao = dimensoes
            .get(indice)
            .ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
        let estimativa =
            estimativa_da_pagina(coberturas_da_selecao[posicao], *dimensao, perfil, qualidade)
                .map_err(|erro| FalhaAnalise::com_detalhe(CodigoFalhaAnalise::Calculo, erro))?;
        consumos.push(estimativa.tinta);
    }
    let estimativa = estimativa_da_selecao(&consumos, copias, perfil)
        .map_err(|erro| FalhaAnalise::com_detalhe(CodigoFalhaAnalise::Calculo, erro))?;
    let estado = cartucho(estimativa.tinta, copias, perfil)
        .map_err(|erro| FalhaAnalise::com_detalhe(CodigoFalhaAnalise::Calculo, erro))?;
    let total_com_copias = selecao
        .quantidade()
        .checked_mul(copias)
        .ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
    Ok(ResumoDaSelecao {
        paginas: selecao.numeros().to_vec(),
        quantidade: selecao.quantidade(),
        total_com_copias,
        tinta: estimativa.tinta.into(),
        custo: estimativa.custo.into(),
        cartucho: estado.into(),
    })
}

/// Monta o resultado da análise sem desenhar nada: recebe a cobertura de cada página e a
/// configuração, e devolve os totais da seleção e do documento.
///
/// Números de página, tamanho da seleção, cópias e dimensões são conferidos antes de qualquer
/// conta. A cobertura não depende do perfil nem da qualidade, então trocá-los muda só a tinta e o
/// custo.
pub fn montar_resultado(
    coberturas: &[Cobertura],
    dimensoes: &[DimensoesPagina],
    selecao: &SelecaoDePaginas,
    copias: usize,
    configuracao: &ConfiguracaoDeTinta,
    perfil: &PerfilDeTinta,
    modo_usado: ModoUsado,
) -> Result<ResultadoDaAnalise, FalhaAnalise> {
    configuracao.validar().map_err(|erro| {
        FalhaAnalise::com_detalhe(CodigoFalhaAnalise::ConfiguracaoInvalida, erro)
    })?;
    perfil.validar().map_err(|erro| {
        FalhaAnalise::com_detalhe(CodigoFalhaAnalise::ConfiguracaoInvalida, erro)
    })?;
    let copias = validar_copias_em_numero(copias)?;
    let total = coberturas.len();
    if total == 0 {
        return Err(FalhaAnalise::novo(CodigoFalhaAnalise::Documento));
    }
    if dimensoes.len() != total || selecao.total() != total {
        return Err(FalhaAnalise::novo(CodigoFalhaAnalise::Calculo));
    }
    if selecao
        .numeros()
        .iter()
        .any(|numero| !(1..=total).contains(numero))
    {
        return Err(FalhaAnalise::novo(CodigoFalhaAnalise::SelecaoInvalida));
    }

    let (consumos, custos) =
        calcular_paginas(coberturas, dimensoes, perfil, configuracao.qualidade)?;

    let mut coberturas_da_selecao = Vec::new();
    coberturas_da_selecao
        .try_reserve_exact(selecao.quantidade())
        .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
    for numero in selecao.numeros() {
        let indice = numero
            .checked_sub(1)
            .ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
        coberturas_da_selecao.push(
            *coberturas
                .get(indice)
                .ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?,
        );
    }

    let resumo_da_selecao = resumo_da_selecao(
        &coberturas_da_selecao,
        dimensoes,
        selecao,
        copias,
        perfil,
        configuracao.qualidade,
    )?;

    let documento = estimativa_do_documento(&consumos, perfil)
        .map_err(|erro| FalhaAnalise::com_detalhe(CodigoFalhaAnalise::Calculo, erro))?;
    let ml_medio_por_pagina = documento.tinta.total() / total as f64;
    if !ml_medio_por_pagina.is_finite() {
        return Err(FalhaAnalise::novo(CodigoFalhaAnalise::Calculo));
    }
    let pagina_mais_cara = pagina_mais_cara(&custos).and_then(|(indice, _)| {
        Some(PaginaMaisCara {
            numero: indice + 1,
            tinta: consumos.get(indice).copied()?.into(),
            custo: custos.get(indice).copied()?.into(),
        })
    });

    Ok(ResultadoDaAnalise {
        total_de_paginas: total,
        modo_usado,
        aviso: None,
        aviso_detalhe: None,
        dpi: configuracao.dpi,
        perfil: perfil.id.clone(),
        moeda: perfil.moeda.clone(),
        qualidade: configuracao.qualidade,
        copias,
        documento_inteiro: selecao.documento_inteiro(),
        selecao: resumo_da_selecao,
        documento: ResumoDoDocumento {
            tinta: documento.tinta.into(),
            custo: documento.custo.into(),
            cobertura_media: media_de_cobertura(coberturas).into(),
            ml_medio_por_pagina,
            pagina_mais_cara,
        },
    })
}

fn cobertura_da_pagina(
    pagina: &PaginaRgb,
    cinzas_so_preto: bool,
) -> Result<Cobertura, FalhaAnalise> {
    let largura = usize::try_from(pagina.largura)
        .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Renderizacao))?;
    let altura = usize::try_from(pagina.altura)
        .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Renderizacao))?;
    cobertura_de_bgra(
        &pagina.pixels,
        largura,
        altura,
        pagina.passo,
        cinzas_so_preto,
    )
    .map_err(|erro| FalhaAnalise::com_detalhe(CodigoFalhaAnalise::Renderizacao, erro))
}

fn resumo_da_selecao_com_cache(
    coberturas: &[Option<Cobertura>],
    dimensoes: &[DimensoesPagina],
    selecao: &SelecaoDePaginas,
    copias: usize,
    perfil: &PerfilDeTinta,
    qualidade: Qualidade,
) -> Result<Option<ResumoDaSelecao>, FalhaAnalise> {
    let mut escolhidas = Vec::new();
    escolhidas
        .try_reserve_exact(selecao.quantidade())
        .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
    for numero in selecao.numeros() {
        let Some(cobertura) = numero
            .checked_sub(1)
            .and_then(|indice| coberturas.get(indice).copied().flatten())
        else {
            return Ok(None);
        };
        escolhidas.push(cobertura);
    }
    resumo_da_selecao(&escolhidas, dimensoes, selecao, copias, perfil, qualidade).map(Some)
}

trait EmissorDeAnalise {
    fn progresso(&self, evento: &ProgressoDaAnalise);
    fn pagina(&self, evento: &PaginaAnalisada);
    fn fim(&self, evento: &FimDaAnalise);
    fn falha(&self, evento: &FalhaDaAnalise);
}

struct EmissorTauri(AppHandle);

impl EmissorDeAnalise for EmissorTauri {
    fn progresso(&self, evento: &ProgressoDaAnalise) {
        let _ = self.0.emit(EVENTO_PROGRESSO, evento);
    }

    fn pagina(&self, evento: &PaginaAnalisada) {
        let _ = self.0.emit(EVENTO_PAGINA, evento);
    }

    fn fim(&self, evento: &FimDaAnalise) {
        let _ = self.0.emit(EVENTO_FIM, evento);
    }

    fn falha(&self, evento: &FalhaDaAnalise) {
        let _ = self.0.emit(EVENTO_FALHA, evento);
    }
}

trait FonteDePaginas {
    fn renderizar(&mut self, numero: usize, dpi: u32) -> Result<PaginaRgb, ErroAnalise>;
}

impl FonteDePaginas for SessaoDeAnalise {
    fn renderizar(&mut self, numero: usize, dpi: u32) -> Result<PaginaRgb, ErroAnalise> {
        SessaoDeAnalise::renderizar(self, numero, dpi)
    }
}

#[derive(Debug)]
enum Desfecho {
    Concluido(ResultadoDaAnalise),
    Cancelado,
    Falhou(FalhaAnalise),
}

fn ordem_da_analise(preparo: &Preparo) -> Result<Vec<usize>, FalhaAnalise> {
    let total = preparo.total();
    let mut vistos = Vec::new();
    vistos
        .try_reserve_exact(total)
        .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Interrompido))?;
    vistos.resize(total, false);
    let mut ordem = Vec::new();
    ordem
        .try_reserve_exact(total)
        .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Interrompido))?;
    let mut inserir = |numero: usize| {
        if let Some(indice) = numero.checked_sub(1).filter(|indice| *indice < total) {
            if !vistos[indice] {
                vistos[indice] = true;
                ordem.push(numero);
            }
        }
    };
    if let Some(atual) = preparo.pagina_atual {
        inserir(atual);
    }
    for numero in preparo.selecao.numeros() {
        inserir(*numero);
    }
    for numero in 1..=total {
        inserir(numero);
    }
    Ok(ordem)
}

/// Teto aproximado de bytes por lote do conversor.
const LOTE_DO_CONVERSOR_BYTES: u64 = 256 * 1024 * 1024;
/// Teto de páginas por lote, mesmo quando as páginas são pequenas.
const LOTE_DO_CONVERSOR_PAGINAS: usize = 32;

/// Divide as páginas que faltam em lotes que cabem no teto de disco do conversor. A estimativa
/// usa a medida de cada página e o dpi; página sem medida vai sozinha, para nunca somar um valor
/// desconhecido ao lote.
fn lotes_do_conversor(
    faltando: &[u32],
    dimensoes: &[DimensoesPagina],
    dpi: u32,
) -> Vec<Vec<u32>> {
    let mut lotes = Vec::new();
    let mut lote = Vec::new();
    let mut bytes = 0u64;
    for pagina in faltando {
        let estimativa = dimensoes
            .get(usize::try_from(*pagina).unwrap_or(usize::MAX).saturating_sub(1))
            .map_or(LOTE_DO_CONVERSOR_BYTES, |medida| {
                bytes_do_pam(medida.largura_mm, medida.altura_mm, dpi)
            });
        let cabe = lote.is_empty()
            || (lote.len() < LOTE_DO_CONVERSOR_PAGINAS
                && bytes.saturating_add(estimativa) <= LOTE_DO_CONVERSOR_BYTES);
        if !cabe {
            lotes.push(std::mem::take(&mut lote));
            bytes = 0;
        }
        lote.push(*pagina);
        bytes = bytes.saturating_add(estimativa);
    }
    if !lote.is_empty() {
        lotes.push(lote);
    }
    lotes
}

/// Bytes aproximados de um PAM CMYK (4 canais) para a medida e o dpi pedidos.
fn bytes_do_pam(largura_mm: f64, altura_mm: f64, dpi: u32) -> u64 {
    let pixels = (largura_mm / 25.4 * f64::from(dpi)).ceil().max(1.0)
        * (altura_mm / 25.4 * f64::from(dpi)).ceil().max(1.0);
    let bytes = pixels * 4.0 + 1024.0;
    if !bytes.is_finite() || bytes >= u64::MAX as f64 {
        u64::MAX
    } else {
        bytes as u64
    }
}

fn executar<F, E>(
    analises: &Analises,
    id: &str,
    preparo: &Preparo,
    cancelamento: &AtomicBool,
    fonte: &mut F,
    emissor: &E,
    conversor: Option<&Path>,
) -> Desfecho
where
    F: FonteDePaginas,
    E: EmissorDeAnalise,
{
    if cancelamento.load(Ordering::Relaxed) {
        return Desfecho::Cancelado;
    }
    let pedida = preparo.chave.clone();
    let mut chave = pedida.clone();
    let mut modo = ModoUsado::Simples;
    let mut aviso = None;
    let mut aviso_detalhe = None;
    let mut coberturas = match analises.abrir_cache(&chave, &preparo.dimensoes) {
        Ok(coberturas) => coberturas,
        Err(falha) => return Desfecho::Falhou(falha),
    };
    if preparo.configuracao.modo == ModoDeCor::Cmyk {
        let completo = coberturas.iter().all(Option::is_some);
        if completo {
            modo = ModoUsado::Cmyk;
        } else if conversor.is_some() {
            modo = ModoUsado::Cmyk;
        } else {
            match trocar_para_simples(analises, &pedida, preparo) {
                Ok((nova_chave, novas)) => {
                    chave = nova_chave;
                    coberturas = novas;
                    aviso = Some(AVISO_SEM_CONVERSOR.to_owned());
                }
                Err(falha) => return Desfecho::Falhou(falha),
            }
        }
    }

    let total = preparo.total();
    let ordem = match ordem_da_analise(preparo) {
        Ok(ordem) => ordem,
        Err(falha) => return Desfecho::Falhou(falha),
    };

    // O conversor trabalha em lotes com teto de bytes estimados: um documento longo a 300 dpi
    // não pode encher o disco com todos os PAMs de uma vez. O resultado fica em memória e entra
    // no cache conforme cada página é publicada; cancelar no meio não guarda um resultado parcial
    // como se o lote tivesse terminado.
    let mut do_conversor: HashMap<usize, Cobertura> = HashMap::new();
    if modo == ModoUsado::Cmyk {
        let mut faltando = Vec::new();
        for numero in &ordem {
            let Some(indice) = numero.checked_sub(1).filter(|indice| *indice < total) else {
                continue;
            };
            if coberturas.get(indice).is_some_and(Option::is_some) {
                continue;
            }
            if let Ok(pagina) = u32::try_from(*numero) {
                faltando.push(pagina);
            }
        }
        if !faltando.is_empty() {
            let lotes = lotes_do_conversor(
                &faltando,
                &preparo.dimensoes,
                preparo.configuracao.dpi,
            );
            let mut falha_do_conversor: Option<ErroMutool> = None;
            for lote in lotes {
                if cancelamento.load(Ordering::Relaxed) {
                    return Desfecho::Cancelado;
                }
                let resultado = match conversor {
                    Some(conversor) => mutool::desenhar_cmyk_com(
                        conversor,
                        &preparo.pdf,
                        &lote,
                        preparo.configuracao.dpi,
                        mutool::LIMITE_PADRAO,
                        cancelamento,
                    ),
                    None => Err(ErroMutool::Temporario {
                        detalhe: "conversor ausente depois da conferência".to_owned(),
                    }),
                };
                match resultado {
                    Ok(paginas) => {
                        let antes = do_conversor.len();
                        for pagina in paginas {
                            if let Some(indice) = usize::try_from(pagina.pagina)
                                .ok()
                                .and_then(|numero| numero.checked_sub(1))
                                .filter(|indice| *indice < total)
                            {
                                do_conversor.insert(indice, pagina.cobertura);
                            }
                        }
                        if do_conversor.len() - antes != lote.len() {
                            falha_do_conversor = Some(ErroMutool::PamFaltando {
                                pagina: lote.first().copied().unwrap_or(1),
                            });
                            break;
                        }
                    }
                    Err(ErroMutool::Cancelado) => return Desfecho::Cancelado,
                    Err(falha) => {
                        falha_do_conversor = Some(falha);
                        break;
                    }
                }
            }
            if let Some(falha) = falha_do_conversor {
                match trocar_para_simples(analises, &pedida, preparo) {
                    Ok((nova_chave, novas)) => {
                        chave = nova_chave;
                        coberturas = novas;
                        modo = ModoUsado::Simples;
                        aviso = Some(AVISO_FALHA_DO_CONVERSOR.to_owned());
                        aviso_detalhe = Some(falha.to_string());
                        do_conversor.clear();
                    }
                    Err(falha) => return Desfecho::Falhou(falha),
                }
            }
        }
    }

    let mut selecionada = Vec::new();
    if selecionada.try_reserve_exact(total).is_err() {
        return Desfecho::Falhou(FalhaAnalise::novo(CodigoFalhaAnalise::Interrompido));
    }
    selecionada.resize(total, false);
    for numero in preparo.selecao.numeros() {
        if let Some(indice) = numero.checked_sub(1).filter(|indice| *indice < total) {
            selecionada[indice] = true;
        }
    }
    let mut faltam_na_selecao = preparo
        .selecao
        .numeros()
        .iter()
        .filter(|numero| {
            numero
                .checked_sub(1)
                .and_then(|indice| coberturas.get(indice))
                .is_none_or(|cobertura| cobertura.is_none())
        })
        .count();
    let mut feitas = coberturas
        .iter()
        .filter(|cobertura| cobertura.is_some())
        .count();

    let inicial = match resumo_da_selecao_com_cache(
        &coberturas,
        &preparo.dimensoes,
        &preparo.selecao,
        preparo.copias,
        &preparo.perfil,
        preparo.configuracao.qualidade,
    ) {
        Ok(resumo) => resumo,
        Err(falha) => return Desfecho::Falhou(falha),
    };
    let mut selecao_emitida = inicial.is_some();
    emissor.progresso(&ProgressoDaAnalise {
        id: id.to_owned(),
        feitas,
        total,
        selecao: inicial,
    });

    for numero in ordem {
        if cancelamento.load(Ordering::Relaxed) {
            return Desfecho::Cancelado;
        }
        let Some(indice) = numero.checked_sub(1).filter(|indice| *indice < total) else {
            continue;
        };
        if coberturas[indice].is_some() {
            continue;
        }
        let cobertura = if let Some(cobertura) = do_conversor.remove(&indice) {
            cobertura
        } else {
            let pagina = match fonte.renderizar(numero, preparo.configuracao.dpi) {
                Ok(pagina) => pagina,
                Err(erro) => return Desfecho::Falhou(FalhaAnalise::da_renderizacao(erro)),
            };
            let cobertura = match cobertura_da_pagina(&pagina, preparo.configuracao.cinzas_so_preto)
            {
                Ok(cobertura) => cobertura,
                Err(falha) => return Desfecho::Falhou(falha),
            };
            drop(pagina);
            cobertura
        };
        analises.gravar_cobertura(&chave, &preparo.dimensoes, indice, cobertura);
        coberturas[indice] = Some(cobertura);
        feitas = feitas.saturating_add(1);

        let dimensao = match preparo.dimensoes.get(indice) {
            Some(dimensao) => *dimensao,
            None => return Desfecho::Falhou(FalhaAnalise::novo(CodigoFalhaAnalise::Calculo)),
        };
        let estimativa = match estimativa_da_pagina(
            cobertura,
            dimensao,
            &preparo.perfil,
            preparo.configuracao.qualidade,
        ) {
            Ok(estimativa) => estimativa,
            Err(erro) => {
                return Desfecho::Falhou(FalhaAnalise::com_detalhe(
                    CodigoFalhaAnalise::Calculo,
                    erro,
                ))
            }
        };
        emissor.pagina(&PaginaAnalisada {
            id: id.to_owned(),
            pagina: numero,
            largura_mm: dimensao.largura_mm,
            altura_mm: dimensao.altura_mm,
            cobertura: cobertura.into(),
            tinta: estimativa.tinta.into(),
            custo: estimativa.custo.into(),
        });
        if selecionada[indice] {
            faltam_na_selecao = faltam_na_selecao.saturating_sub(1);
        }
        let resumo = if faltam_na_selecao == 0 && !selecao_emitida {
            selecao_emitida = true;
            match resumo_da_selecao_com_cache(
                &coberturas,
                &preparo.dimensoes,
                &preparo.selecao,
                preparo.copias,
                &preparo.perfil,
                preparo.configuracao.qualidade,
            ) {
                Ok(resumo) => resumo,
                Err(falha) => return Desfecho::Falhou(falha),
            }
        } else {
            None
        };
        emissor.progresso(&ProgressoDaAnalise {
            id: id.to_owned(),
            feitas,
            total,
            selecao: resumo,
        });
        if cancelamento.load(Ordering::Relaxed) {
            return Desfecho::Cancelado;
        }
    }

    let completas: Option<Vec<Cobertura>> = coberturas.iter().copied().collect();
    if cancelamento.load(Ordering::Relaxed) {
        return Desfecho::Cancelado;
    }
    let Some(completas) = completas else {
        return Desfecho::Falhou(FalhaAnalise::novo(CodigoFalhaAnalise::AindaNaoAnalisada));
    };
    let mut resultado = match montar_resultado(
        &completas,
        &preparo.dimensoes,
        &preparo.selecao,
        preparo.copias,
        &preparo.configuracao,
        &preparo.perfil,
        modo,
    ) {
        Ok(resultado) => resultado,
        Err(falha) => return Desfecho::Falhou(falha),
    };
    resultado.aviso = aviso;
    resultado.aviso_detalhe = aviso_detalhe;
    analises.guardar_concluida(id, chave, preparo);
    Desfecho::Concluido(resultado)
}

/// Reabre o cache no modo simples, onde a análise anterior que caiu no fallback foi guardada.
fn trocar_para_simples(
    analises: &Analises,
    pedida: &ChaveDeCache,
    preparo: &Preparo,
) -> Result<(ChaveDeCache, Vec<Option<Cobertura>>), FalhaAnalise> {
    let chave = chave_do_modo(pedida, ModoDeCor::Simples);
    let coberturas = analises.abrir_cache(&chave, &preparo.dimensoes)?;
    Ok((chave, coberturas))
}

fn publicar<E: EmissorDeAnalise>(emissor: &E, id: &str, desfecho: Desfecho) {
    match desfecho {
        Desfecho::Concluido(resultado) => emissor.fim(&FimDaAnalise {
            id: id.to_owned(),
            resultado,
        }),
        Desfecho::Cancelado => emissor.falha(&FalhaDaAnalise {
            id: id.to_owned(),
            codigo: CodigoFalhaAnalise::Cancelado,
            detalhe: None,
        }),
        Desfecho::Falhou(falha) => emissor.falha(&FalhaDaAnalise {
            id: id.to_owned(),
            codigo: falha.codigo,
            detalhe: falha.detalhe,
        }),
    }
}

struct LimpezaDoTrabalho<'a> {
    analises: &'a Analises,
    id: &'a str,
}

impl Drop for LimpezaDoTrabalho<'_> {
    fn drop(&mut self) {
        self.analises.remover(self.id);
    }
}

impl Analises {
    fn iniciar_trabalho(
        &self,
        app: AppHandle,
        mut preparo: Preparo,
    ) -> Result<String, FalhaAnalise> {
        let id = self.novo_id();
        let cancelamento = Arc::new(AtomicBool::new(false));
        self.registrar(&id, cancelamento.clone());
        let analises = self.clone();
        let id_do_trabalho = id.clone();
        let bytes = std::mem::take(&mut preparo.bytes);
        let iniciado = std::thread::Builder::new()
            .name(format!("analise-de-tinta-{id}"))
            .spawn(move || {
                let id = id_do_trabalho.as_str();
                let _limpeza = LimpezaDoTrabalho {
                    analises: &analises,
                    id,
                };
                let emissor = EmissorTauri(app);
                let mut sessao = match SessaoDeAnalise::abrir(bytes) {
                    Ok(sessao) => sessao,
                    Err(erro) => {
                        let falha = FalhaAnalise::da_renderizacao(erro);
                        emissor.falha(&FalhaDaAnalise {
                            id: id.to_owned(),
                            codigo: falha.codigo,
                            detalhe: falha.detalhe,
                        });
                        return;
                    }
                };
                let desfecho = executar(
                    &analises,
                    id,
                    &preparo,
                    cancelamento.as_ref(),
                    &mut sessao,
                    &emissor,
                    mutool::mutool_do_sistema().as_deref(),
                );
                publicar(&emissor, id, desfecho);
            });
        if iniciado.is_err() {
            self.remover(&id);
            return Err(FalhaAnalise::novo(CodigoFalhaAnalise::IniciarTrabalho));
        }
        Ok(id)
    }

    /// Exporta a última análise concluída: uma linha por página da seleção e uma linha de total.
    fn exportar_csv(&self, id: &str, caminho: &Path) -> Result<(), FalhaAnalise> {
        let retrato = self
            .retrato_concluida(id)
            .ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::AnaliseDesconhecida))?;
        let (dimensoes, coberturas) = self.ler_cache(&retrato.chave).ok_or_else(|| {
            FalhaAnalise::com_detalhe(
                CodigoFalhaAnalise::CacheAusente,
                "o resultado da análise não está mais no cache",
            )
        })?;
        if dimensoes.len() != coberturas.len() {
            return Err(FalhaAnalise::novo(CodigoFalhaAnalise::Calculo));
        }
        let completas: Option<Vec<Cobertura>> = coberturas.iter().copied().collect();
        let Some(completas) = completas else {
            return Err(FalhaAnalise::novo(CodigoFalhaAnalise::AindaNaoAnalisada));
        };
        let mut paginas = Vec::new();
        paginas
            .try_reserve_exact(retrato.selecao.quantidade())
            .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Interrompido))?;
        for numero in retrato.selecao.numeros() {
            let indice = numero
                .checked_sub(1)
                .ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
            let dimensao = *dimensoes
                .get(indice)
                .ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
            let cobertura = *completas
                .get(indice)
                .ok_or_else(|| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
            let estimativa = estimativa_da_pagina(
                cobertura,
                dimensao,
                &retrato.perfil,
                retrato.configuracao.qualidade,
            )
            .map_err(|erro| FalhaAnalise::com_detalhe(CodigoFalhaAnalise::Calculo, erro))?;
            let numero = u32::try_from(*numero)
                .map_err(|_| FalhaAnalise::novo(CodigoFalhaAnalise::Calculo))?;
            paginas.push(PaginaDeTinta {
                numero,
                dimensoes: dimensao,
                cobertura,
                tinta: estimativa.tinta,
                custo: estimativa.custo,
            });
        }
        csv_tinta::gravar_csv(caminho, &SelecaoDeTinta::nova(&paginas, retrato.copias))
            .map_err(|erro| FalhaAnalise::com_detalhe(CodigoFalhaAnalise::Exportacao, erro))
    }
}

/// Começa a análise de um documento. A leitura e a validação acontecem fora da thread da
/// interface; o desenho corre em uma thread própria e devolve o identificador na hora.
#[tauri::command]
pub async fn iniciar_analise(
    app: AppHandle,
    estado: State<'_, Analises>,
    caminho: String,
    paginas: Option<String>,
    copias: u32,
    configuracao: ConfiguracaoDeTinta,
    pagina_atual: Option<usize>,
) -> Result<String, FalhaAnalise> {
    let analises = (*estado).clone();
    let preparo = tauri::async_runtime::spawn_blocking(move || {
        preparar(
            &caminho,
            paginas.as_deref(),
            copias,
            configuracao,
            pagina_atual,
        )
    })
    .await
    .unwrap_or_else(|_| Err(FalhaAnalise::novo(CodigoFalhaAnalise::Interrompido)))?;
    analises.iniciar_trabalho(app, preparo)
}

/// Recalcula uma análise concluída pelo cache, sem desenhar página nenhuma.
#[tauri::command]
pub async fn recalcular_analise(
    estado: State<'_, Analises>,
    id: String,
    configuracao: ConfiguracaoDeTinta,
    paginas: Option<String>,
    copias: u32,
) -> Result<ResultadoDaAnalise, FalhaAnalise> {
    let analises = (*estado).clone();
    tauri::async_runtime::spawn_blocking(move || {
        let perfil = plotinfo::perfil_guardado(&configuracao.perfil).ok_or_else(|| {
            FalhaAnalise::com_detalhe(
                CodigoFalhaAnalise::PerfilNaoEncontrado,
                configuracao.perfil.clone(),
            )
        })?;
        analises.recalcular_com_perfil(&id, &configuracao, paginas.as_deref(), copias, &perfil)
    })
    .await
    .unwrap_or_else(|_| Err(FalhaAnalise::novo(CodigoFalhaAnalise::Interrompido)))
}

/// Pede o cancelamento. `false` quando o trabalho já terminou; não é erro.
#[tauri::command]
pub fn cancelar_analise(estado: State<'_, Analises>, id: String) -> bool {
    estado.cancelar(&id)
}

/// Exporta a última análise concluída para um arquivo CSV. O disco pode demorar, por isso a
/// gravação corre em uma thread própria.
#[tauri::command]
pub async fn exportar_csv(
    estado: State<'_, Analises>,
    id: String,
    caminho: String,
) -> Result<(), FalhaAnalise> {
    let analises = (*estado).clone();
    tauri::async_runtime::spawn_blocking(move || analises.exportar_csv(&id, Path::new(&caminho)))
        .await
        .unwrap_or_else(|_| Err(FalhaAnalise::novo(CodigoFalhaAnalise::Interrompido)))
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicBool;

    static CONTADOR_DE_PASTAS: AtomicU64 = AtomicU64::new(0);

    struct PastaDeTeste(PathBuf);

    impl PastaDeTeste {
        fn nova() -> Self {
            let sequencia = CONTADOR_DE_PASTAS.fetch_add(1, Ordering::Relaxed);
            let caminho = std::env::temp_dir().join(format!(
                "plots-analise-teste-{}-{sequencia}",
                std::process::id()
            ));
            fs::create_dir_all(&caminho).expect("criar pasta do teste");
            Self(caminho)
        }

        fn gravar(&self, nome: &str, bytes: &[u8]) -> PathBuf {
            let caminho = self.0.join(nome);
            fs::write(&caminho, bytes).expect("gravar arquivo do teste");
            caminho
        }
    }

    impl Drop for PastaDeTeste {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn dimensoes(total: usize) -> Vec<DimensoesPagina> {
        vec![DimensoesPagina::new(210.0, 297.0); total]
    }

    fn coberturas_de_teste(total: usize) -> Vec<Cobertura> {
        (0..total)
            .map(|indice| {
                let c = ((indice + 1) * 7 % 101) as f64;
                let k = if indice == 6 { 100.0 } else { 0.0 };
                Cobertura::new(c, 0.0, 0.0, k)
            })
            .collect()
    }

    fn perfil(id: &str, volume: f64, preco: f64) -> PerfilDeTinta {
        PerfilDeTinta {
            id: id.to_owned(),
            nome: format!("Perfil {id}"),
            moeda: "BRL".to_owned(),
            volumes_ml: [volume; 4],
            rendimentos_paginas: [5000; 4],
            precos_ml: [preco; 4],
            fatores_qualidade: [0.6, 1.0, 1.3],
        }
    }

    fn configuracao() -> ConfiguracaoDeTinta {
        ConfiguracaoDeTinta::default()
    }

    fn selecao(texto: &str, total: usize) -> SelecaoDePaginas {
        selecao_de_texto(texto, total).unwrap()
    }

    fn tinta_esperada(
        coberturas: &[Cobertura],
        dimensoes: &[DimensoesPagina],
        paginas: &[usize],
        copias: usize,
        perfil: &PerfilDeTinta,
        qualidade: Qualidade,
    ) -> f64 {
        let soma: f64 = paginas
            .iter()
            .map(|pagina| {
                estimativa_da_pagina(
                    coberturas[pagina - 1],
                    dimensoes[pagina - 1],
                    perfil,
                    qualidade,
                )
                .unwrap()
                .tinta
                .total()
            })
            .sum();
        soma * copias as f64
    }

    fn montar(
        coberturas: &[Cobertura],
        dimensoes: &[DimensoesPagina],
        selecao: &SelecaoDePaginas,
        copias: usize,
        configuracao: &ConfiguracaoDeTinta,
        perfil: &PerfilDeTinta,
    ) -> Result<ResultadoDaAnalise, FalhaAnalise> {
        montar_resultado(
            coberturas,
            dimensoes,
            selecao,
            copias,
            configuracao,
            perfil,
            ModoUsado::Simples,
        )
    }

    #[test]
    fn a_selecao_de_paginas_soltas_soma_so_as_escolhidas() {
        let coberturas = coberturas_de_teste(16);
        let dimensoes = dimensoes(16);
        let perfil = perfil("a", 65.0, 1.0);
        let configuracao = configuracao();
        let escolhidas = selecao("2,5,7,9", 16);
        let resultado = montar(
            &coberturas,
            &dimensoes,
            &escolhidas,
            1,
            &configuracao,
            &perfil,
        )
        .unwrap();
        assert_eq!(resultado.selecao.paginas, vec![2, 5, 7, 9]);
        assert_eq!(resultado.selecao.quantidade, 4);
        assert_eq!(resultado.selecao.total_com_copias, 4);
        assert!(!resultado.documento_inteiro);
        assert_eq!(resultado.modo_usado, ModoUsado::Simples);
        let esperado = tinta_esperada(
            &coberturas,
            &dimensoes,
            &[2, 5, 7, 9],
            1,
            &perfil,
            configuracao.qualidade,
        );
        assert!(
            (resultado.selecao.tinta.total - esperado).abs() < 1e-9,
            "{} × {esperado}",
            resultado.selecao.tinta.total
        );
        assert!(resultado.selecao.cartucho.percentual_consumido > 0.0);
        assert!(resultado.selecao.cartucho.copias_ate_esvaziar.is_some());
    }

    #[test]
    fn uma_pagina_com_duas_copias_dobra_a_tinta() {
        let coberturas = coberturas_de_teste(16);
        let dimensoes = dimensoes(16);
        let perfil = perfil("a", 65.0, 1.0);
        let configuracao = configuracao();
        let escolhidas = selecao("1", 16);
        let uma = montar(
            &coberturas,
            &dimensoes,
            &escolhidas,
            1,
            &configuracao,
            &perfil,
        )
        .unwrap();
        let duas = montar(
            &coberturas,
            &dimensoes,
            &escolhidas,
            2,
            &configuracao,
            &perfil,
        )
        .unwrap();
        assert_eq!(duas.selecao.total_com_copias, 2);
        assert!((duas.selecao.tinta.total - uma.selecao.tinta.total * 2.0).abs() < 1e-9);
        assert!((duas.selecao.custo.total - uma.selecao.custo.total * 2.0).abs() < 1e-9);
    }

    #[test]
    fn uma_faixa_com_doze_copias_conta_paginas_vezes_copias() {
        let coberturas = coberturas_de_teste(16);
        let dimensoes = dimensoes(16);
        let perfil = perfil("a", 65.0, 1.0);
        let configuracao = configuracao();
        let escolhidas = selecao("5-16", 16);
        let resultado = montar(
            &coberturas,
            &dimensoes,
            &escolhidas,
            12,
            &configuracao,
            &perfil,
        )
        .unwrap();
        assert_eq!(resultado.selecao.quantidade, 12);
        assert_eq!(resultado.selecao.total_com_copias, 144);
        let esperado = tinta_esperada(
            &coberturas,
            &dimensoes,
            &(5..=16).collect::<Vec<_>>(),
            12,
            &perfil,
            configuracao.qualidade,
        );
        assert!((resultado.selecao.tinta.total - esperado).abs() < 1e-8);
    }

    #[test]
    fn o_campo_vazio_vale_o_documento_inteiro_em_uma_copia() {
        let coberturas = coberturas_de_teste(16);
        let dimensoes = dimensoes(16);
        let perfil = perfil("a", 65.0, 1.0);
        let configuracao = configuracao();
        let escolhidas = selecao("", 16);
        let resultado = montar(
            &coberturas,
            &dimensoes,
            &escolhidas,
            1,
            &configuracao,
            &perfil,
        )
        .unwrap();
        assert!(resultado.documento_inteiro);
        assert_eq!(resultado.selecao.quantidade, 16);
        assert_eq!(resultado.selecao.total_com_copias, 16);
        assert!((resultado.selecao.tinta.total - resultado.documento.tinta.total).abs() < 1e-9);
        assert!((resultado.selecao.custo.total - resultado.documento.custo.total).abs() < 1e-9);
    }

    #[test]
    fn copias_zero_ou_acima_do_limite_sao_recusadas() {
        let coberturas = coberturas_de_teste(4);
        let dimensoes = dimensoes(4);
        let perfil = perfil("a", 65.0, 1.0);
        let configuracao = configuracao();
        let escolhidas = selecao("", 4);
        for copias in [0, LIMITE_COPIAS + 1] {
            let erro = montar(
                &coberturas,
                &dimensoes,
                &escolhidas,
                copias,
                &configuracao,
                &perfil,
            )
            .unwrap_err();
            assert_eq!(erro.codigo, CodigoFalhaAnalise::CopiasInvalidas, "{copias}");
        }
    }

    #[test]
    fn trocar_o_perfil_muda_a_tinta_e_o_custo_sem_mudar_a_cobertura() {
        let coberturas = coberturas_de_teste(16);
        let dimensoes = dimensoes(16);
        let configuracao = configuracao();
        let escolhidas = selecao("2,5,7,9", 16);
        let barato = montar(
            &coberturas,
            &dimensoes,
            &escolhidas,
            1,
            &configuracao,
            &perfil("a", 65.0, 1.0),
        )
        .unwrap();
        let caro = montar(
            &coberturas,
            &dimensoes,
            &escolhidas,
            1,
            &configuracao,
            &perfil("b", 130.0, 3.0),
        )
        .unwrap();
        assert_eq!(
            barato.documento.cobertura_media,
            caro.documento.cobertura_media
        );
        assert!((caro.selecao.tinta.total - barato.selecao.tinta.total * 2.0).abs() < 1e-9);
        assert!((caro.selecao.custo.total - barato.selecao.custo.total * 6.0).abs() < 1e-9);
    }

    #[test]
    fn a_pagina_mais_cara_e_a_que_soma_mais_tinta() {
        let coberturas = coberturas_de_teste(16);
        let dimensoes = dimensoes(16);
        let perfil = perfil("a", 65.0, 1.0);
        let configuracao = configuracao();
        let resultado = montar(
            &coberturas,
            &dimensoes,
            &selecao("", 16),
            1,
            &configuracao,
            &perfil,
        )
        .unwrap();
        let mais_cara = resultado
            .documento
            .pagina_mais_cara
            .expect("há páginas no documento");
        assert_eq!(mais_cara.numero, 7);
        assert!(mais_cara.custo.total > 0.0);
    }

    #[test]
    fn contagens_incoerentes_e_documento_vazio_sao_recusados() {
        let coberturas = coberturas_de_teste(4);
        let perfil = perfil("a", 65.0, 1.0);
        let configuracao = configuracao();
        let escolhidas = selecao("", 4);

        let erro = montar(
            &coberturas,
            &dimensoes(3),
            &escolhidas,
            1,
            &configuracao,
            &perfil,
        )
        .unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::Calculo);

        let outra_selecao = selecao("", 3);
        let erro = montar(
            &coberturas,
            &dimensoes(4),
            &outra_selecao,
            1,
            &configuracao,
            &perfil,
        )
        .unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::Calculo);

        let vazia = selecao("", 0);
        let erro = montar(&[], &[], &vazia, 1, &configuracao, &perfil).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::Documento);
    }

    fn semear_cache(
        analises: &Analises,
        id: &str,
        chave: ChaveDeCache,
        dimensoes: Vec<DimensoesPagina>,
        coberturas: Vec<Option<Cobertura>>,
    ) {
        let total = dimensoes.len();
        let mut cache = analises
            .0
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        cache.insert(
            chave.clone(),
            EntradaDeCache {
                dimensoes,
                coberturas,
                uso: 1,
            },
        );
        drop(cache);
        let mut concluidas = analises
            .0
            .concluidas
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        concluidas.insert(
            id.to_owned(),
            Concluida {
                chave,
                selecao: selecao("", total),
                copias: 1,
                configuracao: configuracao(),
                perfil: perfil("a", 65.0, 1.0),
                uso: 1,
            },
        );
    }

    #[test]
    fn recalcular_usa_o_cache_e_nao_muda_a_cobertura() {
        let analises = Analises::default();
        let total = 16;
        let dimensoes = dimensoes(total);
        let coberturas = coberturas_de_teste(total);
        let chave = ChaveDeCache {
            impressao: "conteudo".to_owned(),
            dpi: 150,
            modo: 0,
        };
        semear_cache(
            &analises,
            "a1",
            chave,
            dimensoes.clone(),
            coberturas.iter().copied().map(Some).collect(),
        );
        let configuracao = configuracao();
        let p1 = perfil("p1", 65.0, 1.0);
        let p2 = perfil("p2", 130.0, 3.0);
        let primeiro = analises
            .recalcular_com_perfil("a1", &configuracao, Some("2,5,7,9"), 1, &p1)
            .unwrap();
        let mut outra = configuracao.clone();
        outra.perfil = "p2".to_owned();
        outra.qualidade = Qualidade::Alta;
        let segundo = analises
            .recalcular_com_perfil("a1", &outra, Some("2,5,7,9"), 1, &p2)
            .unwrap();
        assert_eq!(
            primeiro.documento.cobertura_media,
            segundo.documento.cobertura_media
        );
        assert!(segundo.selecao.tinta.total > primeiro.selecao.tinta.total);
        assert!(segundo.selecao.custo.total > primeiro.selecao.custo.total);
        assert_eq!(segundo.selecao.paginas, vec![2, 5, 7, 9]);
    }

    #[test]
    fn trocar_dpi_ou_modo_nao_acha_o_cache() {
        let analises = Analises::default();
        let total = 4;
        let coberturas = coberturas_de_teste(total);
        let chave = ChaveDeCache {
            impressao: "conteudo".to_owned(),
            dpi: 150,
            modo: 0,
        };
        semear_cache(
            &analises,
            "a1",
            chave,
            dimensoes(total),
            coberturas.iter().copied().map(Some).collect(),
        );
        let perfil = perfil("p1", 65.0, 1.0);
        let mut com_outro_dpi = configuracao();
        com_outro_dpi.dpi = 300;
        let erro = analises
            .recalcular_com_perfil("a1", &com_outro_dpi, None, 1, &perfil)
            .unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::CacheAusente);

        let mut com_modo_simples = configuracao();
        com_modo_simples.modo = ModoDeCor::Simples;
        let erro = analises
            .recalcular_com_perfil("a1", &com_modo_simples, None, 1, &perfil)
            .unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::CacheAusente);
    }

    #[test]
    fn recalcular_recusa_analise_desconhecida_pagina_invalida_e_cache_parcial() {
        let analises = Analises::default();
        let perfil = perfil("p1", 65.0, 1.0);
        let configuracao = configuracao();
        let erro = analises
            .recalcular_com_perfil("nao-existe", &configuracao, None, 1, &perfil)
            .unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::AnaliseDesconhecida);

        let total = 4;
        let chave = ChaveDeCache {
            impressao: "conteudo".to_owned(),
            dpi: 150,
            modo: 0,
        };
        let mut coberturas = coberturas_de_teste(total)
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();
        coberturas[2] = None;
        semear_cache(&analises, "a1", chave, dimensoes(total), coberturas);
        let erro = analises
            .recalcular_com_perfil("a1", &configuracao, None, 1, &perfil)
            .unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::AindaNaoAnalisada);

        let erro = analises
            .recalcular_com_perfil("a1", &configuracao, Some("9"), 1, &perfil)
            .unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::SelecaoInvalida);
        let erro = analises
            .recalcular_com_perfil("a1", &configuracao, None, 0, &perfil)
            .unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::CopiasInvalidas);
    }

    #[test]
    fn preparar_recusa_configuracao_copias_e_caminho_antes_de_abrir() {
        let mut com_outro_dpi = configuracao();
        com_outro_dpi.dpi = 200;
        let erro = preparar("c:/nao/existe/nada.pdf", None, 1, com_outro_dpi, None).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::ConfiguracaoInvalida);

        let erro = preparar("c:/nao/existe/nada.pdf", None, 0, configuracao(), None).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::CopiasInvalidas);
    }

    #[derive(Default)]
    struct Coletor {
        progressos: Mutex<Vec<ProgressoDaAnalise>>,
        paginas: Mutex<Vec<PaginaAnalisada>>,
        fins: Mutex<Vec<FimDaAnalise>>,
        falhas: Mutex<Vec<FalhaDaAnalise>>,
    }

    impl EmissorDeAnalise for Coletor {
        fn progresso(&self, evento: &ProgressoDaAnalise) {
            self.progressos
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(evento.clone());
        }

        fn pagina(&self, evento: &PaginaAnalisada) {
            self.paginas
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(evento.clone());
        }

        fn fim(&self, evento: &FimDaAnalise) {
            self.fins
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(evento.clone());
        }

        fn falha(&self, evento: &FalhaDaAnalise) {
            self.falhas
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(evento.clone());
        }
    }

    fn pagina_branca(numero: usize) -> PaginaRgb {
        let largura = 8usize;
        let altura = 8usize;
        PaginaRgb {
            numero,
            largura: largura as u32,
            altura: altura as u32,
            passo: largura * 4,
            pixels: vec![255u8; largura * altura * 4],
        }
    }

    struct FonteFalsa {
        cancelamento: Arc<AtomicBool>,
        cancelar_na_chamada: Option<usize>,
        falhar_na_chamada: Option<usize>,
        chamadas: Vec<usize>,
    }

    impl FonteFalsa {
        fn nova(cancelamento: Arc<AtomicBool>) -> Self {
            Self {
                cancelamento,
                cancelar_na_chamada: None,
                falhar_na_chamada: None,
                chamadas: Vec::new(),
            }
        }
    }

    impl FonteDePaginas for FonteFalsa {
        fn renderizar(&mut self, numero: usize, _dpi: u32) -> Result<PaginaRgb, ErroAnalise> {
            self.chamadas.push(numero);
            let chamada = self.chamadas.len();
            if self.cancelar_na_chamada == Some(chamada) {
                self.cancelamento.store(true, Ordering::Relaxed);
            }
            if self.falhar_na_chamada == Some(chamada) {
                return Err(ErroAnalise::Rasterizacao("falha simulada".to_owned()));
            }
            Ok(pagina_branca(numero))
        }
    }

    fn preparo_de_teste(total: usize, texto: &str, pagina_atual: Option<usize>) -> Preparo {
        let mut configuracao = configuracao();
        configuracao.modo = ModoDeCor::Simples;
        Preparo {
            chave: ChaveDeCache {
                impressao: "teste".to_owned(),
                dpi: 150,
                modo: codigo_do_modo(ModoDeCor::Simples),
            },
            bytes: Vec::new(),
            dimensoes: dimensoes(total),
            selecao: selecao(texto, total),
            pagina_atual,
            configuracao,
            perfil: perfil("a", 65.0, 1.0),
            copias: 1,
            pdf: PathBuf::new(),
        }
    }

    #[test]
    fn cancelar_antes_do_primeiro_desenho_termina_em_cancelado_sem_fim() {
        let analises = Analises::default();
        let preparo = preparo_de_teste(6, "", Some(1));
        let cancelamento = Arc::new(AtomicBool::new(true));
        let mut fonte = FonteFalsa::nova(cancelamento.clone());
        let coletor = Coletor::default();
        let desfecho = executar(
            &analises,
            "a1",
            &preparo,
            cancelamento.as_ref(),
            &mut fonte,
            &coletor,
            None,
        );
        assert!(matches!(&desfecho, Desfecho::Cancelado));
        publicar(&coletor, "a1", desfecho);
        assert!(fonte.chamadas.is_empty());
        assert!(coletor
            .fins
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty());
        let falhas = coletor
            .falhas
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert_eq!(falhas.len(), 1);
        assert_eq!(falhas[0].codigo, CodigoFalhaAnalise::Cancelado);
        assert!(analises.chave_concluida("a1").is_none());
    }

    #[test]
    fn cancelar_no_meio_para_sem_entregar_o_fim() {
        let analises = Analises::default();
        let preparo = preparo_de_teste(6, "", Some(1));
        let cancelamento = Arc::new(AtomicBool::new(false));
        let mut fonte = FonteFalsa::nova(cancelamento.clone());
        fonte.cancelar_na_chamada = Some(3);
        let coletor = Coletor::default();
        let desfecho = executar(
            &analises,
            "a1",
            &preparo,
            cancelamento.as_ref(),
            &mut fonte,
            &coletor,
            None,
        );
        assert!(matches!(&desfecho, Desfecho::Cancelado));
        publicar(&coletor, "a1", desfecho);
        assert_eq!(fonte.chamadas.len(), 3);
        assert!(coletor
            .fins
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty());
        let falhas = coletor
            .falhas
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert_eq!(falhas[0].codigo, CodigoFalhaAnalise::Cancelado);
        assert!(analises.chave_concluida("a1").is_none());
        let (_, coberturas) = analises.ler_cache(&preparo.chave).unwrap();
        assert_eq!(
            coberturas
                .iter()
                .filter(|cobertura| cobertura.is_some())
                .count(),
            3
        );
    }

    #[test]
    fn a_analise_completa_entrega_paginas_progresso_e_fim() {
        let analises = Analises::default();
        let preparo = preparo_de_teste(6, "1,2", Some(4));
        let cancelamento = Arc::new(AtomicBool::new(false));
        let mut fonte = FonteFalsa::nova(cancelamento.clone());
        let coletor = Coletor::default();
        let desfecho = executar(
            &analises,
            "a1",
            &preparo,
            cancelamento.as_ref(),
            &mut fonte,
            &coletor,
            None,
        );
        let resultado = match desfecho {
            Desfecho::Concluido(resultado) => resultado,
            outro => panic!("{outro:?}"),
        };
        publicar(&coletor, "a1", Desfecho::Concluido(resultado.clone()));
        assert_eq!(resultado.total_de_paginas, 6);
        assert_eq!(fonte.chamadas, vec![4, 1, 2, 3, 5, 6]);
        let progressos = coletor
            .progressos
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert_eq!(progressos.last().unwrap().feitas, 6);
        assert_eq!(progressos.last().unwrap().total, 6);
        assert!(progressos
            .iter()
            .any(|progresso| progresso.selecao.is_some()));
        let com_selecao = progressos
            .iter()
            .find(|progresso| progresso.selecao.is_some())
            .unwrap();
        assert!(com_selecao.feitas < com_selecao.total);
        assert_eq!(com_selecao.selecao.as_ref().unwrap().paginas, vec![1, 2]);
        assert_eq!(
            coletor
                .fins
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .len(),
            1
        );
        assert!(analises.chave_concluida("a1").is_some());
    }

    #[test]
    fn a_segunda_analise_reaproveita_as_paginas_do_cache() {
        let analises = Analises::default();
        let preparo = preparo_de_teste(4, "", Some(1));
        let cancelamento = Arc::new(AtomicBool::new(false));
        let mut primeira = FonteFalsa::nova(cancelamento.clone());
        let coletor = Coletor::default();
        let desfecho = executar(
            &analises,
            "a1",
            &preparo,
            cancelamento.as_ref(),
            &mut primeira,
            &coletor,
            None,
        );
        assert!(matches!(&desfecho, Desfecho::Concluido(_)));
        assert_eq!(primeira.chamadas.len(), 4);

        let mut segunda = FonteFalsa::nova(cancelamento.clone());
        let coletor = Coletor::default();
        let desfecho = executar(
            &analises,
            "a2",
            &preparo,
            cancelamento.as_ref(),
            &mut segunda,
            &coletor,
            None,
        );
        assert!(matches!(&desfecho, Desfecho::Concluido(_)));
        assert!(segunda.chamadas.is_empty());
        let progressos = coletor
            .progressos
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert_eq!(progressos.first().unwrap().feitas, 4);
    }

    #[test]
    fn a_falha_de_uma_pagina_termina_em_falha_e_guarda_o_que_ja_foi_lido() {
        let analises = Analises::default();
        let preparo = preparo_de_teste(4, "", Some(1));
        let cancelamento = Arc::new(AtomicBool::new(false));
        let mut fonte = FonteFalsa::nova(cancelamento.clone());
        fonte.falhar_na_chamada = Some(2);
        let coletor = Coletor::default();
        let desfecho = executar(
            &analises,
            "a1",
            &preparo,
            cancelamento.as_ref(),
            &mut fonte,
            &coletor,
            None,
        );
        assert!(matches!(&desfecho, Desfecho::Falhou(_)));
        publicar(&coletor, "a1", desfecho);
        assert!(coletor
            .fins
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty());
        let falhas = coletor
            .falhas
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert_eq!(falhas[0].codigo, CodigoFalhaAnalise::Renderizacao);
        assert!(analises.chave_concluida("a1").is_none());
        let (_, coberturas) = analises.ler_cache(&preparo.chave).unwrap();
        assert_eq!(
            coberturas
                .iter()
                .filter(|cobertura| cobertura.is_some())
                .count(),
            1
        );
    }

    fn preparo_cmyk(total: usize, texto: &str, pdf: PathBuf) -> Preparo {
        let configuracao = configuracao();
        Preparo {
            chave: chave_de("teste-cmyk", &configuracao),
            bytes: Vec::new(),
            dimensoes: dimensoes(total),
            selecao: selecao(texto, total),
            pagina_atual: None,
            configuracao,
            perfil: perfil("a", 65.0, 1.0),
            copias: 1,
            pdf,
        }
    }

    fn paginas_em_csv(linhas: &[&str]) -> Vec<String> {
        linhas
            .iter()
            .skip(1)
            .filter(|linha| !linha.starts_with("total;"))
            .map(|linha| linha.split(';').next().unwrap_or_default().to_owned())
            .collect()
    }

    #[test]
    fn sem_conversor_a_analise_usa_o_modo_simples_com_aviso() {
        let analises = Analises::default();
        // O caminho não existe, mas o conversor também não: nada tenta abrir o PDF.
        let preparo = preparo_cmyk(4, "2,3", PathBuf::from("nao-existe.pdf"));
        let cancelamento = Arc::new(AtomicBool::new(false));
        let mut fonte = FonteFalsa::nova(cancelamento.clone());
        let coletor = Coletor::default();
        let desfecho = executar(
            &analises,
            "a1",
            &preparo,
            cancelamento.as_ref(),
            &mut fonte,
            &coletor,
            None,
        );
        let resultado = match desfecho {
            Desfecho::Concluido(resultado) => resultado,
            outro => panic!("{outro:?}"),
        };
        assert_eq!(resultado.modo_usado, ModoUsado::Simples);
        assert_eq!(resultado.aviso.as_deref(), Some(AVISO_SEM_CONVERSOR));
        assert_eq!(fonte.chamadas, vec![2, 3, 1, 4]);
        assert_eq!(
            analises.chave_concluida("a1").map(|chave| chave.modo),
            Some(codigo_do_modo(ModoDeCor::Simples))
        );
        let (_, coberturas) = analises.ler_cache(&preparo.chave).unwrap();
        assert!(coberturas.iter().all(Option::is_none));
    }

    #[test]
    fn conversor_que_falha_cai_no_simples_e_o_recalculo_continua_simples() {
        let pasta = PastaDeTeste::nova();
        let pdf = pasta.gravar(
            "tres.pdf",
            &crate::mutool::testes::pdf_cmyk(&[0.1, 0.2, 0.3]),
        );
        let analises = Analises::default();
        let preparo = preparo_cmyk(3, "", pdf);
        let cancelamento = Arc::new(AtomicBool::new(false));
        let mut fonte = FonteFalsa::nova(cancelamento.clone());
        let coletor = Coletor::default();
        let conversor = Path::new("c:/nao/existe/mutool.exe");
        let desfecho = executar(
            &analises,
            "a1",
            &preparo,
            cancelamento.as_ref(),
            &mut fonte,
            &coletor,
            Some(conversor),
        );
        let resultado = match desfecho {
            Desfecho::Concluido(resultado) => resultado,
            outro => panic!("{outro:?}"),
        };
        assert_eq!(resultado.modo_usado, ModoUsado::Simples);
        assert_eq!(resultado.aviso.as_deref(), Some(AVISO_FALHA_DO_CONVERSOR));
        assert!(
            resultado
                .aviso_detalhe
                .as_deref()
                .is_some_and(|detalhe| detalhe.contains("mutool")),
            "{:?}",
            resultado.aviso_detalhe
        );
        assert_eq!(fonte.chamadas, vec![1, 2, 3]);
        assert_eq!(
            analises.chave_concluida("a1").map(|chave| chave.modo),
            Some(codigo_do_modo(ModoDeCor::Simples))
        );

        let recalculado = analises
            .recalcular_com_perfil("a1", &preparo.configuracao, None, 1, &preparo.perfil)
            .expect("a análise no modo simples continua recalculável");
        assert_eq!(recalculado.modo_usado, ModoUsado::Simples);
        assert_eq!(recalculado.aviso.as_deref(), Some(AVISO_MODO_SIMPLES));

        let mut simples = preparo.configuracao.clone();
        simples.modo = ModoDeCor::Simples;
        let recalculado = analises
            .recalcular_com_perfil("a1", &simples, None, 1, &preparo.perfil)
            .expect("o modo simples explícito também lê o cache");
        assert_eq!(recalculado.modo_usado, ModoUsado::Simples);
        assert_eq!(recalculado.aviso, None);
    }

    #[test]
    fn cancelar_antes_do_conversor_nao_entrega_fim_nem_grava_cache() {
        let analises = Analises::default();
        let preparo = preparo_cmyk(3, "", PathBuf::from("nao-existe.pdf"));
        let cancelamento = Arc::new(AtomicBool::new(true));
        let mut fonte = FonteFalsa::nova(cancelamento.clone());
        let coletor = Coletor::default();
        let desfecho = executar(
            &analises,
            "a1",
            &preparo,
            cancelamento.as_ref(),
            &mut fonte,
            &coletor,
            Some(Path::new("c:/nao/existe/mutool.exe")),
        );
        assert!(matches!(&desfecho, Desfecho::Cancelado));
        publicar(&coletor, "a1", desfecho);
        assert!(fonte.chamadas.is_empty());
        assert!(coletor
            .fins
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty());
        let falhas = coletor
            .falhas
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert_eq!(falhas[0].codigo, CodigoFalhaAnalise::Cancelado);
        assert!(analises.chave_concluida("a1").is_none());
        assert!(analises.ler_cache(&preparo.chave).is_none());
    }

    #[test]
    fn csv_da_selecao_tem_uma_linha_por_pagina_e_total_com_copias() {
        let analises = Analises::default();
        let mut preparo = preparo_de_teste(16, "2,5,7,9", None);
        preparo.copias = 2;
        let cancelamento = Arc::new(AtomicBool::new(false));
        let mut fonte = FonteFalsa::nova(cancelamento.clone());
        let coletor = Coletor::default();
        let desfecho = executar(
            &analises,
            "a1",
            &preparo,
            cancelamento.as_ref(),
            &mut fonte,
            &coletor,
            None,
        );
        let resultado = match desfecho {
            Desfecho::Concluido(resultado) => resultado,
            outro => panic!("{outro:?}"),
        };
        assert_eq!(resultado.selecao.paginas, vec![2, 5, 7, 9]);
        assert_eq!(resultado.selecao.total_com_copias, 8);

        let pasta = PastaDeTeste::nova();
        let destino = pasta.0.join("tinta.csv");
        analises
            .exportar_csv("a1", &destino)
            .expect("a seleção deveria virar CSV");
        let texto = fs::read_to_string(&destino).expect("ler o CSV");
        let linhas: Vec<&str> = texto.lines().collect();
        assert_eq!(linhas.len(), 6, "cabeçalho, quatro páginas e total");
        assert_eq!(linhas[0], csv_tinta::CABECALHO);
        assert_eq!(paginas_em_csv(&linhas), vec!["2", "5", "7", "9"]);

        let total: Vec<&str> = linhas[5].split(';').collect();
        assert_eq!(total.len(), 13);
        assert_eq!(total[0], "total");
        let total_ml: f64 = total[11].parse().expect("total em mL");
        let custo: f64 = total[12].parse().expect("custo total");
        assert!((total_ml - resultado.selecao.tinta.total).abs() < 1e-6);
        assert!((custo - resultado.selecao.custo.total).abs() < 1e-6);

        // Cada linha de página mostra uma cópia; o total já repete pelas duas cópias.
        let soma_de_uma_copia: f64 = linhas[1..5]
            .iter()
            .map(|linha| {
                linha
                    .split(';')
                    .nth(11)
                    .expect("total em mL da página")
                    .parse::<f64>()
                    .expect("número")
            })
            .sum();
        assert!((total_ml - soma_de_uma_copia * 2.0).abs() < 1e-6);

        // Um recálculo com outra seleção e outras cópias atualiza o retrato usado na exportação.
        let recalculado = analises
            .recalcular_com_perfil("a1", &preparo.configuracao, Some("1"), 3, &preparo.perfil)
            .expect("a nova seleção deveria ser recalculada pelo cache");
        assert_eq!(recalculado.selecao.total_com_copias, 3);
        let segundo = pasta.0.join("tinta-recalculada.csv");
        analises
            .exportar_csv("a1", &segundo)
            .expect("o retrato novo deveria ser exportado");
        let segundo = fs::read_to_string(&segundo).expect("ler o segundo CSV");
        let linhas: Vec<&str> = segundo.lines().collect();
        assert_eq!(linhas.len(), 3);
        assert_eq!(paginas_em_csv(&linhas), vec!["1"]);
    }

    #[test]
    fn csv_recusa_analise_desconhecida_e_destino_invalido_sem_apagar_nada() {
        let analises = Analises::default();
        let pasta = PastaDeTeste::nova();
        let destino = pasta.0.join("tinta.csv");
        let erro = analises.exportar_csv("nao-existe", &destino).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::AnaliseDesconhecida);
        assert!(!destino.exists());

        let preparo = preparo_de_teste(4, "", None);
        let cancelamento = Arc::new(AtomicBool::new(false));
        let mut fonte = FonteFalsa::nova(cancelamento.clone());
        let coletor = Coletor::default();
        let desfecho = executar(
            &analises,
            "a1",
            &preparo,
            cancelamento.as_ref(),
            &mut fonte,
            &coletor,
            None,
        );
        assert!(matches!(&desfecho, Desfecho::Concluido(_)));

        let subpasta = pasta.0.join("destino");
        fs::create_dir(&subpasta).expect("criar pasta de destino");
        let dentro = subpasta.join("importante.txt");
        fs::write(&dentro, "não apagar").expect("gravar arquivo dentro da pasta");
        let erro = analises.exportar_csv("a1", &subpasta).unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::Exportacao);
        assert_eq!(fs::read_to_string(&dentro).unwrap(), "não apagar");
    }

    #[test]
    #[ignore = "depende do mutool real em PLOTS_MUTOOL"]
    fn conversor_real_entrega_cmyk_e_exporta_o_csv() {
        let Some(conversor) = mutool::mutool_do_sistema() else {
            eprintln!("PLOTS_MUTOOL não está configurado; teste do conversor real ignorado");
            return;
        };
        let pasta = PastaDeTeste::nova();
        let pdf = pasta.gravar(
            "três páginas ação.pdf",
            &crate::mutool::testes::pdf_cmyk(&[0.1, 0.2, 0.3]),
        );
        let analises = Analises::default();
        let preparo = preparo_cmyk(3, "", pdf);
        let cancelamento = Arc::new(AtomicBool::new(false));
        let mut fonte = FonteFalsa::nova(cancelamento.clone());
        let coletor = Coletor::default();
        let desfecho = executar(
            &analises,
            "real",
            &preparo,
            cancelamento.as_ref(),
            &mut fonte,
            &coletor,
            Some(&conversor),
        );
        let resultado = match desfecho {
            Desfecho::Concluido(resultado) => resultado,
            outro => panic!("{outro:?}"),
        };
        assert!(fonte.chamadas.is_empty(), "o PDFium não deveria desenhar");
        assert_eq!(resultado.modo_usado, ModoUsado::Cmyk);
        assert_eq!(resultado.aviso, None);

        let paginas = coletor
            .paginas
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert_eq!(
            paginas
                .iter()
                .map(|pagina| pagina.pagina)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        let esperado = [
            25.0 / 255.0 * 100.0,
            51.0 / 255.0 * 100.0,
            76.0 / 255.0 * 100.0,
        ];
        for (pagina, esperado) in paginas.iter().zip(esperado) {
            // A 150 dpi a borda da página cai entre pixels e o antialias muda a média em
            // centésimos; o teste do conversor a 72 dpi confere o byte exato de cada canal.
            assert!(
                (pagina.cobertura.c - esperado).abs() < 0.05,
                "página {}: ciano {:.4}, esperado {:.4}",
                pagina.pagina,
                pagina.cobertura.c,
                esperado
            );
            assert_eq!(pagina.cobertura.m, 0.0);
            assert_eq!(pagina.cobertura.y, 0.0);
            assert_eq!(pagina.cobertura.k, 0.0);
        }
        let medias: Vec<f64> = paginas.iter().map(|pagina| pagina.cobertura.c).collect();
        drop(paginas);

        let destino = pasta.0.join("tinta.csv");
        analises
            .exportar_csv("real", &destino)
            .expect("o resultado CMYK deveria virar CSV");
        let texto = fs::read_to_string(&destino).expect("ler o CSV");
        println!("CSV da análise real:\n{texto}");
        let linhas: Vec<&str> = texto.lines().collect();
        assert_eq!(linhas.len(), 1 + 3 + 1);
        let percentuais: Vec<f64> = linhas[1..4]
            .iter()
            .map(|linha| {
                linha
                    .split(';')
                    .nth(3)
                    .expect("coluna do ciano")
                    .parse()
                    .expect("percentual")
            })
            .collect();
        let alvos = [
            25.0 / 255.0 * 100.0,
            51.0 / 255.0 * 100.0,
            76.0 / 255.0 * 100.0,
        ];
        for (indice, percentual) in percentuais.iter().enumerate() {
            let alvo = alvos[indice];
            assert!(
                (percentual - alvo).abs() < 0.05,
                "página {}: CSV {percentual}, esperado {alvo}",
                indice + 1
            );
            assert!((percentual - medias[indice]).abs() < 1e-4);
        }
        assert!(linhas[4].starts_with("total;"));

        let recalculado = analises
            .recalcular_com_perfil("real", &preparo.configuracao, None, 1, &preparo.perfil)
            .expect("o cache CMYK deveria ser recalculável");
        assert_eq!(recalculado.modo_usado, ModoUsado::Cmyk);
        assert_eq!(recalculado.aviso, None);

        let mut simples = preparo.configuracao.clone();
        simples.modo = ModoDeCor::Simples;
        let erro = analises
            .recalcular_com_perfil("real", &simples, None, 1, &preparo.perfil)
            .unwrap_err();
        assert_eq!(erro.codigo, CodigoFalhaAnalise::CacheAusente);
    }

    #[test]
    fn o_conversor_recebe_lotes_limitados_por_bytes_e_por_paginas() {
        let a4 = DimensoesPagina::new(210.0, 297.0);
        let dimensoes = vec![a4; 100];
        let paginas: Vec<u32> = (1..=100).collect();
        let lotes = lotes_do_conversor(&paginas, &dimensoes, 300);
        assert!(lotes.len() > 1);
        assert_eq!(
            lotes.iter().flatten().copied().collect::<Vec<_>>(),
            paginas
        );
        let estimativa = bytes_do_pam(210.0, 297.0, 300);
        for lote in &lotes {
            assert!(lote.len() <= LOTE_DO_CONVERSOR_PAGINAS);
            let total = (lote.len() as u64).saturating_mul(estimativa);
            assert!(total <= LOTE_DO_CONVERSOR_BYTES, "{total} bytes");
        }
    }

    #[test]
    fn paginas_pequenas_cabem_no_mesmo_lote_ate_o_teto_de_quantidade() {
        let dimensoes = vec![DimensoesPagina::new(10.0, 10.0); 40];
        let paginas: Vec<u32> = (1..=40).collect();
        let lotes = lotes_do_conversor(&paginas, &dimensoes, 72);
        assert_eq!(lotes.len(), 2);
        assert_eq!(lotes[0].len(), LOTE_DO_CONVERSOR_PAGINAS);
        assert_eq!(lotes[1].len(), 40 - LOTE_DO_CONVERSOR_PAGINAS);
    }

    #[test]
    fn pagina_sem_medida_ou_com_medida_invalida_vai_sozinha_no_lote() {
        let sem_medida =
            lotes_do_conversor(&[1, 2, 3], &[DimensoesPagina::new(210.0, 297.0)], 300);
        assert_eq!(sem_medida, vec![vec![1], vec![2], vec![3]]);
        let invalida = vec![DimensoesPagina::new(f64::NAN, f64::INFINITY); 3];
        let lotes = lotes_do_conversor(&[1, 2, 3], &invalida, 300);
        assert_eq!(lotes, vec![vec![1], vec![2], vec![3]]);
    }

    #[test]
    fn a_estimativa_do_pam_acompanha_a_medida_e_o_dpi() {
        let a4_150 = bytes_do_pam(210.0, 297.0, 150);
        let a4_300 = bytes_do_pam(210.0, 297.0, 300);
        assert!((8_000_000..9_000_000).contains(&a4_150), "{a4_150}");
        assert!((32_000_000..36_000_000).contains(&a4_300), "{a4_300}");
    }
}
