//! A bandeja de uma impressora virtual: recebe as passadas, cumpre o gesto no modelo físico, confere
//! o maço e descreve cada folha para a tela. Quem julga é sempre o modelo físico.
//!
//! O modelo trabalha com a identidade de cada página recebida, não com o número que ela traz: assim
//! a folha existe e aparece na tela mesmo quando não dá para ler a página. O número só entra na hora
//! de conferir, e só nas folhas em que ele foi lido.

use crate::marca::PaginaVista;
use crate::vista::{imagens_da_folha, Quadro};
use plotsim_core::canal::LeituraPedida;
use plotsim_core::{FaceDeSaida, ModoFisico, Saida};
use plotsim_physics::{
    conferir, conferir_por_folha, folhas_do_documento, ler_maco, Entrada, Envio, ErroGesto, Face,
    FolhaLida, Gesto, Giro, Idioma, Impressora, Lado, Leitura, MacoLido, ModoImpressora, Motivo,
    Operacao, Pilha, TopoDaImagem, Vetor,
};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

/// Maior número de páginas em uma bandeja. É um simulador de bancada; além disso, o PDF não é teste.
pub const LIMITE_PAGINAS: usize = 4_000;
/// Quantos trabalhos podem esperar o gesto. Depois disso, o trabalho é recusado com aviso.
const LIMITE_PENDENTES: usize = 64;
const LARGURA_PADRAO_MM: f64 = 210.0;
const ALTURA_PADRAO_MM: f64 = 297.0;

/// O modo guardado com a fila, escrito como o modelo físico o entende. A entrada e a saída ficam do
/// mesmo lado da máquina, e o caminho do papel sai da face de saída.
pub fn modo_do_registro(modo: ModoFisico) -> ModoImpressora {
    ModoImpressora {
        entrada: match modo.saida {
            Saida::Traseira => Entrada::Traseira,
            Saida::Gaveta => Entrada::Gaveta,
        },
        puxa: Lado::Cima,
        imprime_na_face_para: Lado::Cima,
        sai_com_face_para: match modo.face {
            FaceDeSaida::ParaCima => Lado::Cima,
            FaceDeSaida::ParaBaixo => Lado::Baixo,
        },
        cai: Lado::Cima,
        topo_da_imagem: TopoDaImagem::NaArestaQueEntra,
    }
}

/// Em que ponto o lote em andamento está.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Etapa {
    /// Nenhum lote em andamento.
    Livre,
    /// A primeira passada está na saída, à espera do gesto.
    AguardandoGesto,
    /// A pilha está na entrada, à espera da segunda passada.
    NaEntrada,
}

/// Algo que a pessoa precisa saber sobre esta impressora.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Aviso {
    pub codigo: &'static str,
    pub detalhe: Option<String>,
}

impl Aviso {
    pub fn novo(codigo: &'static str, detalhe: Option<String>) -> Self {
        Self { codigo, detalhe }
    }
}

struct PaginaGuardada {
    arquivo: Arc<PathBuf>,
    vista: PaginaVista,
}

struct Passada {
    arquivo: Arc<PathBuf>,
    paginas: Vec<PaginaVista>,
    /// É a frente de um lote do PlotS: ela nunca é o verso do lote que está na bandeja.
    e_frente: bool,
}

/// Uma camada de imagem em uma folha da tela.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CamadaTela {
    /// Identifica a página ao pedir o desenho.
    pub pagina: u32,
    /// Número lido; sem ele, a tela não mostra número nenhum.
    pub numero: Option<u32>,
    /// Página em branco: não tem marca de propósito, e não conta como página sem marca.
    pub branca: bool,
    pub graus: u16,
    /// Vista através do papel: espelhada e a 30%.
    pub atras: bool,
    pub largura_mm: f64,
    pub altura_mm: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SituacaoDaFolha {
    /// Ainda falta passada: não há o que conferir.
    EmAndamento,
    Certa,
    Errada,
    SemMarca,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolhaTela {
    pub numero: u32,
    pub situacao: SituacaoDaFolha,
    pub motivos: Vec<String>,
    /// Como a folha aparece e como aparece depois de "Virar folha".
    pub frente: Vec<CamadaTela>,
    pub verso: Vec<CamadaTela>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtapaTela {
    Vazia,
    AguardandoGesto,
    NaEntrada,
    Conferida,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperacaoTela {
    VirarPelaBordaCurta,
    VirarPelaBordaLonga,
    Girar,
    InverterOrdem,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LeituraTela {
    ComoSaiu,
    VirandoOMaco,
}

/// Tudo o que a tela precisa saber da bandeja.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BandejaTela {
    pub etapa: EtapaTela,
    pub recebendo: bool,
    /// Trabalhos que chegaram antes do gesto.
    pub esperando: usize,
    pub folhas: Vec<FolhaTela>,
    pub corretas: u32,
    pub sem_marca: u32,
    /// Folhas conferidas, contando as que faltam. Zero quando nada foi conferido.
    pub conferidas: u32,
    /// Motivos que valem para o maço inteiro.
    pub gerais: Vec<String>,
    pub leitura: Option<LeituraTela>,
    /// Sem instrução do PlotS: a forma foi escolhida pela bandeja, que deu menos motivos.
    pub sem_instrucao: bool,
    pub operacoes: Vec<OperacaoTela>,
    pub paginas_sem_folha: usize,
    pub folhas_sem_verso: usize,
    pub aviso: Option<Aviso>,
    /// O trabalho não é um documento inteiro que começa na página 1: cada folha se confere sozinha.
    pub conferencia_por_folha: bool,
}

/// O gesto não foi cumprido; a pilha ficou onde estava.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecusaDoGesto {
    NadaParaRecolocar,
    Fisica(ErroGesto),
}

pub struct Bandeja {
    modo: ModoImpressora,
    impressora: Impressora,
    etapa: Etapa,
    /// Lotes já concluídos, na ordem em que saíram.
    pacotes: Vec<Pilha>,
    paginas: HashMap<u32, PaginaGuardada>,
    proxima: u32,
    pendentes: VecDeque<Passada>,
    operacoes: Vec<Operacao>,
    paginas_sem_folha: usize,
    folhas_sem_verso: usize,
    recebendo: bool,
    aviso: Option<Aviso>,
    /// Como o PlotS mandou ler o maço, quando ele já terminou o envio desta fila.
    leitura_pedida: Option<LeituraPedida>,
}

/// Como ler o maço, segundo a instrução do PlotS. `SemInstrucao` segue a face que a máquina deixa.
fn leitura_da_instrucao(pedida: LeituraPedida, modo: ModoImpressora) -> Leitura {
    match pedida {
        LeituraPedida::ComoSaiu => Leitura::ComoSaiu,
        LeituraPedida::Virando => Leitura::VirandoOMaco,
        LeituraPedida::SemInstrucao => match modo.sai_com_face_para {
            Lado::Cima => Leitura::ComoSaiu,
            Lado::Baixo => Leitura::VirandoOMaco,
        },
    }
}

fn folha_do_motivo(motivo: Motivo) -> Option<u32> {
    match motivo {
        Motivo::FolhaAMenos { .. } | Motivo::FolhaAMais { .. } => None,
        Motivo::VersoDeCabecaParaBaixo { folha }
        | Motivo::PaginaGirada { folha, .. }
        | Motivo::FolhaDeCabecaParaBaixo { folha }
        | Motivo::FrenteTrocada { folha, .. }
        | Motivo::VersoTrocado { folha, .. }
        | Motivo::FolhaAoContrario { folha }
        | Motivo::PaginaEstranha { folha, .. }
        | Motivo::FaltaPagina { folha, .. }
        | Motivo::ImpressaoSobreImpressao { folha }
        | Motivo::VersoForaDoPar { folha, .. } => Some(folha),
    }
}

fn normal(lado: Lado) -> Vetor {
    match lado {
        Lado::Cima => Vetor::Z,
        Lado::Baixo => Vetor::Z.oposto(),
    }
}

fn outra_face(face: Face) -> Face {
    match face {
        Face::A => Face::B,
        Face::B => Face::A,
    }
}

fn u32_saturado(valor: usize) -> u32 {
    u32::try_from(valor).unwrap_or(u32::MAX)
}

/// Resultado da conferência dos lotes concluídos em uma das formas de pegar o maço.
struct Conferencia {
    leitura: Leitura,
    /// Por folha do maço lido: motivos e se havia marca.
    motivos: HashMap<u32, Vec<Motivo>>,
    sem_marca: Vec<bool>,
    gerais: Vec<Motivo>,
    esperadas: u32,
    /// Cada folha se confere sozinha: o trabalho não é um documento inteiro a partir da página 1.
    por_folha: bool,
}

impl Conferencia {
    /// Alguma página foi lida neste maço? Sem nenhuma, não há forma de pegar que tenha sido posta à prova.
    fn alguma_pagina_lida(&self) -> bool {
        self.sem_marca.contains(&false)
    }

    fn quantos_motivos(&self) -> usize {
        self.gerais
            .len()
            .saturating_add(self.motivos.values().map(Vec::len).sum())
    }
}

impl Bandeja {
    pub fn nova(modo: ModoImpressora) -> Self {
        Self {
            modo,
            impressora: Impressora::nova(modo, 0),
            etapa: Etapa::Livre,
            pacotes: Vec::new(),
            paginas: HashMap::new(),
            proxima: 1,
            pendentes: VecDeque::new(),
            operacoes: Vec::new(),
            paginas_sem_folha: 0,
            folhas_sem_verso: 0,
            recebendo: false,
            aviso: None,
            leitura_pedida: None,
        }
    }

    /// Mesma impressora, sem nada na bandeja nem na entrada.
    pub fn limpar(&mut self) {
        let recebendo = self.recebendo;
        *self = Self::nova(self.modo);
        self.recebendo = recebendo;
    }

    pub fn definir_recebendo(&mut self, recebendo: bool) {
        self.recebendo = recebendo;
    }

    pub fn definir_aviso(&mut self, aviso: Option<Aviso>) {
        self.aviso = aviso;
    }

    pub fn aviso(&self) -> Option<&Aviso> {
        self.aviso.as_ref()
    }

    pub fn recebendo(&self) -> bool {
        self.recebendo
    }

    /// Falta a segunda passada do lote em andamento?
    pub fn aguardando_verso(&self) -> bool {
        self.etapa != Etapa::Livre
    }

    /// Uma passada chegou. Se o gesto ainda não veio, ela espera: a pilha continua na saída.
    pub fn receber(&mut self, arquivo: PathBuf, paginas: Vec<PaginaVista>) {
        self.guardar(arquivo, paginas, false);
    }

    fn guardar(&mut self, arquivo: PathBuf, paginas: Vec<PaginaVista>, e_frente: bool) {
        let esperando: usize = self.pendentes.iter().map(|p| p.paginas.len()).sum();
        let ja_recebidas = self.paginas.len().saturating_add(esperando);
        if ja_recebidas.saturating_add(paginas.len()) > LIMITE_PAGINAS {
            self.aviso = Some(Aviso::novo("bandeja_cheia", None));
            return;
        }
        if self.pendentes.len() >= LIMITE_PENDENTES {
            self.aviso = Some(Aviso::novo("muitos_trabalhos", None));
            return;
        }
        // A frente de um lote novo, com o anterior sem verso e nada esperando o gesto, fecha o
        // anterior: não vira o verso dele.
        if e_frente && self.etapa == Etapa::AguardandoGesto && self.pendentes.is_empty() {
            self.fechar_lote_sem_verso();
        }
        self.pendentes.push_back(Passada {
            arquivo: Arc::new(arquivo),
            paginas,
            e_frente,
        });
        self.andar();
    }

    /// Chegou a frente de um lote do PlotS (`PlotS - lote N - frente`).
    pub fn receber_frente(&mut self, arquivo: PathBuf, paginas: Vec<PaginaVista>) {
        self.guardar(arquivo, paginas, true);
    }

    /// O lote em andamento não vai receber verso: o que saiu fica no maço como está, e a bandeja
    /// fica livre para o lote seguinte.
    fn fechar_lote_sem_verso(&mut self) {
        let sem_verso = match self.etapa {
            Etapa::Livre => return,
            Etapa::AguardandoGesto => {
                let pacote = self.impressora.retirar_saida();
                let folhas = pacote.folhas().len();
                self.pacotes.push(pacote);
                folhas
            }
            // As folhas recolocadas voltam à mão de quem imprimiu: não entram no maço.
            Etapa::NaEntrada => self.impressora.entrada().len(),
        };
        self.folhas_sem_verso = self.folhas_sem_verso.saturating_add(sem_verso);
        self.impressora = Impressora::nova(self.modo, 0);
        self.operacoes.clear();
        self.etapa = Etapa::Livre;
    }

    /// Cumpre o gesto, venha da escolha da pessoa na tela ou de um canal com o programa testado.
    pub fn recolocar(&mut self, gesto: Gesto) -> Result<(), RecusaDoGesto> {
        if self.etapa != Etapa::AguardandoGesto {
            return Err(RecusaDoGesto::NadaParaRecolocar);
        }
        let operacoes = self
            .impressora
            .recolocar(gesto)
            .map_err(RecusaDoGesto::Fisica)?;
        self.operacoes = operacoes;
        self.etapa = Etapa::NaEntrada;
        self.andar();
        Ok(())
    }

    /// O PlotS terminou o envio: guarda como ele pede que o maço seja lido. Pode chegar antes de a
    /// última passada ser recebida; o veredito é refeito a cada consulta.
    pub fn encerrar(&mut self, leitura: LeituraPedida) {
        self.leitura_pedida = Some(leitura);
    }

    /// Imprime o que estiver esperando, até faltar o gesto ou acabar a fila.
    fn andar(&mut self) {
        while self.etapa != Etapa::AguardandoGesto {
            let Some(passada) = self.pendentes.pop_front() else {
                return;
            };
            if passada.e_frente {
                self.fechar_lote_sem_verso();
            }
            let Some(envios) = self.registrar(&passada) else {
                self.aviso = Some(Aviso::novo("bandeja_cheia", None));
                return;
            };
            match self.etapa {
                Etapa::Livre => {
                    // Um lote novo começa: a instrução do lote anterior não vale para ele.
                    self.leitura_pedida = None;
                    self.impressora.abastecer(envios.len());
                    let resumo = self.impressora.imprimir(&envios);
                    self.paginas_sem_folha = self
                        .paginas_sem_folha
                        .saturating_add(resumo.sem_folha.len());
                    self.etapa = Etapa::AguardandoGesto;
                }
                Etapa::NaEntrada => self.imprimir_segunda_passada(&envios),
                Etapa::AguardandoGesto => return,
            }
        }
    }

    fn imprimir_segunda_passada(&mut self, envios: &[Envio]) {
        let resumo = self.impressora.imprimir(envios);
        self.paginas_sem_folha = self
            .paginas_sem_folha
            .saturating_add(resumo.sem_folha.len());
        self.folhas_sem_verso = self
            .folhas_sem_verso
            .saturating_add(self.impressora.entrada().len());
        let pacote = self.impressora.retirar_saida();
        self.pacotes.push(pacote);
        // O que sobrou na entrada não foi para a saída: não entra no maço.
        self.impressora = Impressora::nova(self.modo, 0);
        self.operacoes.clear();
        self.etapa = Etapa::Livre;
    }

    fn registrar(&mut self, passada: &Passada) -> Option<Vec<Envio>> {
        let mut envios = Vec::with_capacity(passada.paginas.len());
        for pagina in &passada.paginas {
            let identidade = self.proxima;
            self.proxima = identidade.checked_add(1)?;
            self.paginas.insert(
                identidade,
                PaginaGuardada {
                    arquivo: Arc::clone(&passada.arquivo),
                    vista: *pagina,
                },
            );
            envios.push(Envio::girada(identidade, pagina.giro));
        }
        Some(envios)
    }

    /// Arquivo e posição de uma página, para desenhá-la.
    pub fn origem_da_pagina(&self, pagina: u32) -> Option<(PathBuf, usize)> {
        let guardada = self.paginas.get(&pagina)?;
        Some((guardada.arquivo.as_ref().clone(), guardada.vista.indice))
    }

    fn numero(&self, pagina: u32) -> Option<u32> {
        self.paginas.get(&pagina).and_then(|p| p.vista.numero)
    }

    fn giro(&self, pagina: u32) -> Giro {
        self.paginas
            .get(&pagina)
            .map_or(Giro::Nenhum, |p| p.vista.giro)
    }

    fn branca(&self, pagina: u32) -> bool {
        self.paginas.get(&pagina).is_some_and(|p| p.vista.branca)
    }

    fn camadas(&self, folha: &plotsim_physics::Folha, quadro: Quadro) -> Vec<CamadaTela> {
        imagens_da_folha(folha, quadro, &|pagina| self.giro(pagina))
            .into_iter()
            .map(|imagem| {
                let vista = self.paginas.get(&imagem.pagina).map(|p| p.vista);
                CamadaTela {
                    pagina: imagem.pagina,
                    numero: vista.and_then(|v| v.numero),
                    branca: vista.is_some_and(|v| v.branca),
                    graus: imagem.graus,
                    atras: imagem.atras,
                    largura_mm: vista.map_or(LARGURA_PADRAO_MM, |v| v.largura_mm),
                    altura_mm: vista.map_or(ALTURA_PADRAO_MM, |v| v.altura_mm),
                }
            })
            .collect()
    }

    /// Soma dos lotes concluídos, lidos da forma pedida.
    fn maco(&self, leitura: Leitura) -> MacoLido {
        let mut maco = MacoLido::default();
        for pacote in &self.pacotes {
            maco.folhas.extend(ler_maco(pacote, leitura).folhas);
        }
        maco
    }

    /// Quantas páginas o documento tem. Conta só as páginas com número lido: uma sem marca legível
    /// não tem número, e nunca é palpite.
    fn total_do_documento(&self, maco: &MacoLido) -> u32 {
        let numeros: Vec<u32> = maco
            .folhas
            .iter()
            .flat_map(|folha| folha.frente.iter().chain(folha.verso.iter()))
            .filter_map(|pagina| self.numero(*pagina))
            .collect();
        let maior = numeros.iter().copied().max().unwrap_or(0);
        maior.max(u32_saturado(numeros.len()))
    }

    /// Troca a identidade pelo número lido. A face sem marca recebe o número que a posição dela
    /// exigiria, só para as outras folhas não mudarem de lugar: o motivo dessa folha é descartado.
    fn traduzir(&self, maco: &MacoLido, total: u32) -> (MacoLido, Vec<bool>) {
        let mut sem_marca = Vec::with_capacity(maco.folhas.len());
        let mut folhas = Vec::with_capacity(maco.folhas.len());
        for (posicao, folha) in maco.folhas.iter().enumerate() {
            let numero = u32_saturado(posicao).saturating_add(1);
            let impar = numero.saturating_mul(2).saturating_sub(1);
            let par = numero.saturating_mul(2);
            let mut faltou = false;
            let mut face = |paginas: &[u32], lugar: u32| -> Vec<u32> {
                paginas
                    .iter()
                    .filter_map(|pagina| {
                        // A página em branco do total ímpar não tem número: não entra no maço lido,
                        // e a folha dela não é tratada como sem marca.
                        if self.branca(*pagina) {
                            return None;
                        }
                        let lido = self.numero(*pagina);
                        faltou |= lido.is_none();
                        lido.or((lugar <= total).then_some(lugar))
                    })
                    .collect()
            };
            let frente = face(&folha.frente, impar);
            let verso = face(&folha.verso, par);
            sem_marca.push(faltou);
            folhas.push(FolhaLida {
                frente,
                verso,
                ..folha.clone()
            });
        }
        (MacoLido { folhas }, sem_marca)
    }

    /// O trabalho não é um documento inteiro que começa na página 1: as marcas não começam na 1,
    /// repetem-se (cópias) ou há mais de um lote (vários papéis). Com a instrução do `Fim`, o
    /// PlotS garante que é o documento e vale a conferência do maço de sempre.
    ///
    /// Limite conhecido: um documento inteiro sem a folha 1 (ela se perdeu ou não foi lida) cai
    /// na conferência por folha, mesmo sem instrução do PlotS que diga que o trabalho é parcial.
    fn conferir_por_folha_vale(&self, maco: &MacoLido) -> bool {
        if self.leitura_pedida.is_some() {
            return false;
        }
        let mut numeros: Vec<u32> = maco
            .folhas
            .iter()
            .flat_map(|folha| folha.frente.iter().chain(folha.verso.iter()))
            .filter_map(|pagina| self.numero(*pagina))
            .collect();
        if numeros.is_empty() {
            return false;
        }
        numeros.sort_unstable();
        let repete = numeros.windows(2).any(|par| par[0] == par[1]);
        self.pacotes.len() > 1 || repete || numeros.first() != Some(&1)
    }

    fn conferir_com(&self, leitura: Leitura) -> Conferencia {
        let lido = self.maco(leitura);
        let total = self.total_do_documento(&lido);
        let (traduzido, sem_marca) = self.traduzir(&lido, total);
        let por_folha = self.conferir_por_folha_vale(&lido);
        let mut motivos: HashMap<u32, Vec<Motivo>> = HashMap::new();
        let mut gerais = Vec::new();
        let ha_sem_marca = sem_marca.contains(&true);
        let encontrados = if por_folha {
            conferir_por_folha(&traduzido)
        } else {
            conferir(&traduzido, total)
        };
        for motivo in encontrados {
            match folha_do_motivo(motivo) {
                // O total sai só das páginas lidas: a folha sem marca pode ser uma das que faltam
                // nessa conta, então não dá para dizer que ela sobra.
                None if ha_sem_marca && matches!(motivo, Motivo::FolhaAMais { .. }) => {}
                None => gerais.push(motivo),
                Some(folha) => {
                    let posicao = usize::try_from(folha.saturating_sub(1)).unwrap_or(usize::MAX);
                    if sem_marca.get(posicao) != Some(&true) {
                        motivos.entry(folha).or_default().push(motivo);
                    }
                }
            }
        }
        let folhas_lidas = u32_saturado(sem_marca.len());
        Conferencia {
            leitura,
            motivos,
            sem_marca,
            gerais,
            esperadas: if por_folha {
                folhas_lidas
            } else {
                folhas_do_documento(total)
            },
            por_folha,
        }
    }

    /// Com a instrução do PlotS, o maço é lido só da forma que ela manda. Sem ela (sem canal),
    /// vale a forma que der menos motivos.
    fn conferir(&self) -> Option<Conferencia> {
        if self.pacotes.is_empty() {
            return None;
        }
        if let Some(pedida) = self.leitura_pedida {
            return Some(self.conferir_com(leitura_da_instrucao(pedida, self.modo)));
        }
        let como_saiu = self.conferir_com(Leitura::ComoSaiu);
        let virando = self.conferir_com(Leitura::VirandoOMaco);
        Some(if virando.quantos_motivos() < como_saiu.quantos_motivos() {
            virando
        } else {
            como_saiu
        })
    }

    /// As folhas prontas, de cima para baixo, como a pessoa as lê.
    fn folhas_conferidas(&self, conferencia: &Conferencia, idioma: Idioma) -> Vec<FolhaTela> {
        let mut folhas = Vec::new();
        let mut cima = None;
        for pacote in &self.pacotes {
            let mut maco = pacote.clone();
            if conferencia.leitura == Leitura::VirandoOMaco {
                maco.aplicar(Operacao::VirarPelaArestaLonga);
            }
            for folha in maco.folhas().iter().rev() {
                let numero = u32_saturado(folhas.len()).saturating_add(1);
                let direcao = *cima.get_or_insert_with(|| direcao_de_leitura(folha));
                let quadro = Quadro::novo(Vetor::Z, direcao);
                let motivos = conferencia.motivos.get(&numero);
                let posicao = folhas.len();
                let situacao = if conferencia.sem_marca.get(posicao) == Some(&true) {
                    SituacaoDaFolha::SemMarca
                } else if motivos.is_some_and(|lista| !lista.is_empty()) {
                    SituacaoDaFolha::Errada
                } else {
                    SituacaoDaFolha::Certa
                };
                folhas.push(FolhaTela {
                    numero,
                    situacao,
                    motivos: motivos
                        .map(|lista| lista.iter().map(|m| m.texto(idioma)).collect())
                        .unwrap_or_default(),
                    frente: self.camadas(folha, quadro),
                    verso: self.camadas(folha, quadro.do_outro_lado()),
                });
            }
        }
        folhas
    }

    /// As folhas do lote em andamento. Antes do gesto, a pessoa vê o lado impresso de cada folha;
    /// depois dele, olha pelo lado em que a máquina vai imprimir.
    fn folhas_em_andamento(&self, inicio: u32) -> Vec<FolhaTela> {
        let pilha = match self.etapa {
            Etapa::Livre => return Vec::new(),
            Etapa::AguardandoGesto => self.impressora.saida(),
            Etapa::NaEntrada => self.impressora.entrada(),
        };
        let cima_da_maquina = match self.modo.topo_da_imagem {
            TopoDaImagem::NaArestaQueEntra => Vetor::Y,
            TopoDaImagem::NaArestaQueSai => Vetor::Y.oposto(),
        };
        let quadro_da_maquina =
            Quadro::novo(normal(self.modo.imprime_na_face_para), cima_da_maquina);
        pilha
            .folhas()
            .iter()
            .rev()
            .enumerate()
            .map(|(posicao, folha)| {
                let quadro = if self.etapa == Etapa::AguardandoGesto {
                    quadro_do_lado_impresso(folha).unwrap_or(quadro_da_maquina)
                } else {
                    quadro_da_maquina
                };
                FolhaTela {
                    numero: inicio.saturating_add(u32_saturado(posicao)),
                    situacao: SituacaoDaFolha::EmAndamento,
                    motivos: Vec::new(),
                    frente: self.camadas(folha, quadro),
                    verso: self.camadas(folha, quadro.do_outro_lado()),
                }
            })
            .collect()
    }

    pub fn tela(&self, idioma: Idioma) -> BandejaTela {
        let conferencia = self.conferir();
        let mut folhas = conferencia
            .as_ref()
            .map(|c| self.folhas_conferidas(c, idioma))
            .unwrap_or_default();
        let corretas = folhas
            .iter()
            .filter(|f| f.situacao == SituacaoDaFolha::Certa)
            .count();
        let sem_marca = folhas
            .iter()
            .filter(|f| f.situacao == SituacaoDaFolha::SemMarca)
            .count();
        let conferidas = u32_saturado(folhas.len()).max(conferencia.as_ref().map_or(0, |c| {
            if sem_marca == 0 {
                c.esperadas
            } else {
                0
            }
        }));
        // Sem nenhuma página lida, nenhuma das formas de pegar o maço foi posta à prova.
        let leitura = conferencia
            .as_ref()
            .filter(|c| c.alguma_pagina_lida())
            .map(|c| match c.leitura {
                Leitura::ComoSaiu => LeituraTela::ComoSaiu,
                Leitura::VirandoOMaco => LeituraTela::VirandoOMaco,
            });
        let inicio = u32_saturado(folhas.len()).saturating_add(1);
        folhas.extend(self.folhas_em_andamento(inicio));
        let etapa = match self.etapa {
            Etapa::AguardandoGesto => EtapaTela::AguardandoGesto,
            Etapa::NaEntrada => EtapaTela::NaEntrada,
            Etapa::Livre if self.pacotes.is_empty() => EtapaTela::Vazia,
            Etapa::Livre => EtapaTela::Conferida,
        };
        BandejaTela {
            etapa,
            recebendo: self.recebendo,
            esperando: self.pendentes.len(),
            folhas,
            corretas: u32_saturado(corretas),
            sem_marca: u32_saturado(sem_marca),
            conferidas,
            gerais: conferencia
                .as_ref()
                .map(|c| c.gerais.iter().map(|m| m.texto(idioma)).collect())
                .unwrap_or_default(),
            sem_instrucao: leitura.is_some() && self.leitura_pedida.is_none(),
            leitura,
            operacoes: self
                .operacoes
                .iter()
                .map(|operacao| match operacao {
                    Operacao::VirarPelaArestaCurta => OperacaoTela::VirarPelaBordaCurta,
                    Operacao::VirarPelaArestaLonga => OperacaoTela::VirarPelaBordaLonga,
                    Operacao::Girar180 => OperacaoTela::Girar,
                    Operacao::InverterFolhaAFolha => OperacaoTela::InverterOrdem,
                })
                .collect(),
            paginas_sem_folha: self.paginas_sem_folha,
            folhas_sem_verso: self.folhas_sem_verso,
            aviso: self.aviso.clone(),
            conferencia_por_folha: conferencia.as_ref().is_some_and(|c| c.por_folha),
        }
    }
}

/// Para onde aponta o topo da imagem que a pessoa lê primeiro nesta folha: a de cima ou, se a de
/// cima estiver em branco, a de baixo.
fn direcao_de_leitura(folha: &plotsim_physics::Folha) -> Vetor {
    let face = folha.face_para(Lado::Cima);
    folha
        .impressoes(face)
        .first()
        .or_else(|| folha.impressoes(outra_face(face)).first())
        .map_or(Vetor::Y, |impressao| folha.topo_no_mundo(*impressao))
}

/// Olha pelo lado em que a folha foi impressa, com a imagem de pé.
fn quadro_do_lado_impresso(folha: &plotsim_physics::Folha) -> Option<Quadro> {
    let lado = [Lado::Cima, Lado::Baixo]
        .into_iter()
        .find(|lado| !folha.impressoes(folha.face_para(*lado)).is_empty())?;
    let impressao = folha.impressoes(folha.face_para(lado)).first()?;
    Some(Quadro::novo(normal(lado), folha.topo_no_mundo(*impressao)))
}

#[cfg(test)]
mod testes {
    use super::*;
    use plotsim_physics::{BordaQueEntra, FaceNaEntrada};

    const JATO: ModoFisico = ModoFisico {
        saida: Saida::Traseira,
        face: FaceDeSaida::ParaCima,
    };
    const LASER: ModoFisico = ModoFisico {
        saida: Saida::Gaveta,
        face: FaceDeSaida::ParaBaixo,
    };
    const GESTO_JATO: Gesto = Gesto {
        entrada: Entrada::Traseira,
        face_impressa: FaceNaEntrada::ParaTras,
        borda_que_entra: BordaQueEntra::Base,
        inverter_ordem: false,
    };
    const GESTO_LASER: Gesto = Gesto {
        entrada: Entrada::Gaveta,
        face_impressa: FaceNaEntrada::ParaBaixo,
        borda_que_entra: BordaQueEntra::Base,
        inverter_ordem: false,
    };

    fn pagina(indice: usize, numero: Option<u32>, giro: Giro) -> PaginaVista {
        PaginaVista {
            indice,
            numero,
            branca: false,
            giro,
            largura_mm: 210.0,
            altura_mm: 297.0,
        }
    }

    fn em_branco(indice: usize) -> PaginaVista {
        PaginaVista {
            branca: true,
            ..pagina(indice, None, Giro::Nenhum)
        }
    }

    fn numeradas(numeros: &[u32], giro: Giro) -> Vec<PaginaVista> {
        numeros
            .iter()
            .enumerate()
            .map(|(indice, numero)| pagina(indice, Some(*numero), giro))
            .collect()
    }

    fn sem_numero(quantas: usize, giro: Giro) -> Vec<PaginaVista> {
        (0..quantas).map(|i| pagina(i, None, giro)).collect()
    }

    fn bandeja_com(modo: ModoFisico) -> Bandeja {
        Bandeja::nova(modo_do_registro(modo))
    }

    fn jato_certo() -> Bandeja {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4, 2], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[5, 3, 1], Giro::Meia180));
        bandeja
    }

    #[test]
    fn os_modos_guardados_viram_as_duas_maquinas_do_modelo() {
        let jato = modo_do_registro(JATO);
        assert_eq!(jato.entrada, Entrada::Traseira);
        assert_eq!(jato.sai_com_face_para, Lado::Cima);
        let laser = modo_do_registro(LASER);
        assert_eq!(laser.entrada, Entrada::Gaveta);
        assert_eq!(laser.sai_com_face_para, Lado::Baixo);
    }

    #[test]
    fn bandeja_nova_esta_vazia_sem_conferencia() {
        let tela = bandeja_com(JATO).tela(Idioma::PtBr);
        assert_eq!(tela.etapa, EtapaTela::Vazia);
        assert!(tela.folhas.is_empty());
        assert_eq!(tela.conferidas, 0);
        assert_eq!(tela.leitura, None);
    }

    #[test]
    fn primeira_passada_vira_folhas_de_pe_e_sem_veredito() {
        let mut bandeja = bandeja_com(LASER);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4, 2], Giro::Nenhum));
        let tela = bandeja.tela(Idioma::PtBr);
        assert_eq!(tela.etapa, EtapaTela::AguardandoGesto);
        assert_eq!(tela.folhas.len(), 3);
        for folha in &tela.folhas {
            assert_eq!(folha.situacao, SituacaoDaFolha::EmAndamento);
            let [camada] = folha.frente.as_slice() else {
                panic!("uma camada só: {:?}", folha.frente);
            };
            assert_eq!((camada.graus, camada.atras), (0, false));
        }
        assert!(bandeja.aguardando_verso());
    }

    #[test]
    fn na_recolocacao_o_impresso_passa_para_tras_e_de_cabeca_para_baixo_se_o_gesto_girar() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[2], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        let tela = bandeja.tela(Idioma::PtBr);
        assert_eq!(tela.etapa, EtapaTela::NaEntrada);
        assert_eq!(tela.operacoes, [OperacaoTela::VirarPelaBordaCurta]);
        let camada = &tela.folhas[0].frente[0];
        assert!(camada.atras);
        assert_eq!(camada.graus, 180);
    }

    #[test]
    fn o_mesmo_gesto_com_a_borda_trocada_nao_deixa_o_impresso_de_ponta_cabeca() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[2], Giro::Nenhum));
        bandeja
            .recolocar(Gesto {
                borda_que_entra: BordaQueEntra::Topo,
                ..GESTO_JATO
            })
            .unwrap();
        let camada = bandeja.tela(Idioma::PtBr).folhas[0].frente[0].clone();
        assert!(camada.atras);
        assert_eq!(camada.graus, 0);
    }

    #[test]
    fn jato_com_o_gesto_certo_fecha_tudo_com_a_frente_impar_de_pe() {
        let tela = jato_certo().tela(Idioma::PtBr);
        assert_eq!(tela.etapa, EtapaTela::Conferida);
        assert_eq!((tela.corretas, tela.conferidas, tela.sem_marca), (3, 3, 0));
        assert!(tela.gerais.is_empty());
        for (posicao, folha) in tela.folhas.iter().enumerate() {
            assert_eq!(folha.situacao, SituacaoDaFolha::Certa, "{folha:?}");
            let impar = u32::try_from(posicao).unwrap() * 2 + 1;
            let [atras, perto] = folha.frente.as_slice() else {
                panic!("duas camadas: {:?}", folha.frente);
            };
            assert_eq!(perto.numero, Some(impar));
            assert!(!perto.atras);
            assert_eq!(atras.numero, Some(impar + 1));
            assert!(atras.atras);
            // A página ímpar chegou girada no arquivo, e a tela desfaz o giro para mostrá-la de pé.
            assert_eq!(perto.graus, 180);
            assert_eq!(atras.graus, 0);
        }
    }

    #[test]
    fn virar_folha_mostra_a_par_de_frente_e_a_impar_espelhada() {
        let tela = jato_certo().tela(Idioma::PtBr);
        let [atras, perto] = tela.folhas[0].verso.as_slice() else {
            panic!("duas camadas: {:?}", tela.folhas[0].verso);
        };
        assert_eq!(perto.numero, Some(2));
        assert!(!perto.atras);
        assert_eq!(perto.graus, 0);
        assert_eq!(atras.numero, Some(1));
        assert!(atras.atras);
        assert_eq!(atras.graus, 180);
    }

    #[test]
    fn laser_com_o_gesto_certo_fecha_tudo_virando_o_maco() {
        let mut bandeja = bandeja_com(LASER);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4, 2], Giro::Nenhum));
        bandeja.recolocar(GESTO_LASER).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[1, 3, 5], Giro::Meia180));
        let tela = bandeja.tela(Idioma::En);
        assert_eq!((tela.corretas, tela.conferidas), (3, 3));
        assert_eq!(tela.leitura, Some(LeituraTela::VirandoOMaco));
    }

    #[test]
    fn gesto_errado_reprova_cada_folha_com_o_motivo_no_idioma_pedido() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4, 2], Giro::Nenhum));
        bandeja
            .recolocar(Gesto {
                borda_que_entra: BordaQueEntra::Topo,
                ..GESTO_JATO
            })
            .unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[5, 3, 1], Giro::Meia180));
        let portugues = bandeja.tela(Idioma::PtBr);
        assert_eq!(portugues.corretas, 0);
        assert_eq!(portugues.folhas[0].situacao, SituacaoDaFolha::Errada);
        assert!(portugues.folhas[0]
            .motivos
            .iter()
            .any(|motivo| motivo.contains("verso de cabeça para baixo")));
        assert!(bandeja.tela(Idioma::En).folhas[0]
            .motivos
            .iter()
            .any(|motivo| motivo.contains("back upside down")));
    }

    #[test]
    fn sem_marca_a_folha_diz_isso_e_nao_recebe_veredito() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), sem_numero(2, Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), sem_numero(2, Giro::Nenhum));
        let tela = bandeja.tela(Idioma::PtBr);
        assert_eq!(tela.folhas.len(), 2);
        assert_eq!((tela.corretas, tela.sem_marca), (0, 2));
        for folha in &tela.folhas {
            assert_eq!(folha.situacao, SituacaoDaFolha::SemMarca);
            assert!(folha.motivos.is_empty());
            assert!(folha.frente.iter().all(|c| c.numero.is_none()));
        }
    }

    #[test]
    fn sem_nenhuma_marca_nao_ha_veredito_sobre_o_maco() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), sem_numero(3, Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), sem_numero(3, Giro::Nenhum));
        let tela = bandeja.tela(Idioma::PtBr);
        assert!(tela.gerais.is_empty(), "{:?}", tela.gerais);
        assert_eq!(tela.leitura, None);
    }

    #[test]
    fn folha_sem_marca_nao_conta_como_folha_a_mais() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), sem_numero(3, Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        // Só uma página tem número: o total lido é pequeno, e as outras folhas podem ser dele.
        let mut segunda = sem_numero(3, Giro::Nenhum);
        segunda[2].numero = Some(1);
        bandeja.receber("b.pdf".into(), segunda);
        let tela = bandeja.tela(Idioma::PtBr);
        assert!(tela.gerais.is_empty(), "{:?}", tela.gerais);
    }

    #[test]
    fn uma_pagina_sem_marca_nao_tira_o_veredito_das_outras_folhas() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4, 2], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        let mut segunda = numeradas(&[5, 3, 1], Giro::Meia180);
        segunda[1].numero = None;
        bandeja.receber("b.pdf".into(), segunda);
        let tela = bandeja.tela(Idioma::PtBr);
        assert_eq!((tela.corretas, tela.sem_marca), (2, 1));
        assert_eq!(tela.folhas[1].situacao, SituacaoDaFolha::SemMarca);
    }

    #[test]
    fn pagina_em_branco_do_total_impar_nao_e_sem_marca() {
        // Cinco páginas: a última folha tem a frente 5 e o verso em branco, que completa o total.
        let mut bandeja = bandeja_com(JATO);
        let mut primeira = numeradas(&[4, 2], Giro::Nenhum);
        primeira.insert(0, em_branco(0));
        bandeja.receber("a.pdf".into(), primeira);
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[5, 3, 1], Giro::Meia180));
        let tela = bandeja.tela(Idioma::PtBr);
        assert_eq!(
            (tela.corretas, tela.conferidas, tela.sem_marca),
            (3, 3, 0),
            "{tela:?}"
        );
        assert!(tela.gerais.is_empty(), "{:?}", tela.gerais);
        // Cada folha lista as duas faces, então a mesma página aparece em mais de uma lista.
        let brancas: std::collections::BTreeSet<u32> = tela
            .folhas
            .iter()
            .flat_map(|folha| folha.frente.iter().chain(folha.verso.iter()))
            .filter(|camada| camada.branca)
            .map(|camada| camada.pagina)
            .collect();
        assert_eq!(brancas.len(), 1, "{tela:?}");
    }

    #[test]
    fn folha_a_menos_e_acusada_no_maco_inteiro() {
        // Folhas (1,2) e (5,6): a do meio nao saiu. Ha pagina 1 e uma so copia de cada marca.
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 2], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[5, 1], Giro::Meia180));
        let tela = bandeja.tela(Idioma::PtBr);
        assert!(!tela.conferencia_por_folha, "{tela:?}");
        assert!(tela.corretas < tela.conferidas, "{tela:?}");
        assert!(tela.gerais.iter().any(|m| m.contains("folha a menos")));
    }

    #[test]
    fn verso_com_menos_paginas_diz_quantas_folhas_ficaram_na_entrada() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4, 2], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[5, 3], Giro::Meia180));
        assert_eq!(bandeja.tela(Idioma::PtBr).folhas_sem_verso, 1);
    }

    #[test]
    fn verso_que_chega_antes_do_gesto_espera_e_imprime_ao_recolocar() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4, 2], Giro::Nenhum));
        bandeja.receber("b.pdf".into(), numeradas(&[5, 3, 1], Giro::Meia180));
        let antes = bandeja.tela(Idioma::PtBr);
        assert_eq!(antes.etapa, EtapaTela::AguardandoGesto);
        assert_eq!(antes.esperando, 1);
        bandeja.recolocar(GESTO_JATO).unwrap();
        let depois = bandeja.tela(Idioma::PtBr);
        assert_eq!(depois.etapa, EtapaTela::Conferida);
        assert_eq!((depois.esperando, depois.corretas), (0, 3));
    }

    #[test]
    fn gesto_impossivel_deixa_a_pilha_onde_esta() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[2], Giro::Nenhum));
        let erro = bandeja.recolocar(GESTO_LASER).unwrap_err();
        assert!(matches!(
            erro,
            RecusaDoGesto::Fisica(ErroGesto::EntradaDiferente { .. })
        ));
        assert_eq!(bandeja.tela(Idioma::PtBr).etapa, EtapaTela::AguardandoGesto);
    }

    #[test]
    fn recolocar_sem_primeira_passada_e_recusado() {
        let mut bandeja = bandeja_com(JATO);
        assert_eq!(
            bandeja.recolocar(GESTO_JATO),
            Err(RecusaDoGesto::NadaParaRecolocar)
        );
    }

    #[test]
    fn lotes_sao_juntados_na_ordem_em_que_saem() {
        let mut bandeja = bandeja_com(JATO);
        for (par, impar) in [(2, 1), (4, 3)] {
            bandeja.receber("a.pdf".into(), numeradas(&[par], Giro::Nenhum));
            bandeja.recolocar(GESTO_JATO).unwrap();
            bandeja.receber("b.pdf".into(), numeradas(&[impar], Giro::Meia180));
        }
        let tela = bandeja.tela(Idioma::PtBr);
        assert_eq!((tela.corretas, tela.conferidas), (2, 2), "{tela:?}");
        assert_eq!(tela.folhas[1].numero, 2);
    }

    #[test]
    fn limpar_volta_ao_vazio_e_as_paginas_deixam_de_existir() {
        let mut bandeja = jato_certo();
        assert!(bandeja.origem_da_pagina(1).is_some());
        bandeja.limpar();
        assert_eq!(bandeja.tela(Idioma::PtBr).etapa, EtapaTela::Vazia);
        assert_eq!(bandeja.origem_da_pagina(1), None);
    }

    #[test]
    fn pdf_grande_demais_vira_aviso_sem_mexer_na_bandeja() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), sem_numero(LIMITE_PAGINAS + 1, Giro::Nenhum));
        let tela = bandeja.tela(Idioma::PtBr);
        assert_eq!(tela.etapa, EtapaTela::Vazia);
        assert_eq!(tela.aviso.map(|a| a.codigo), Some("bandeja_cheia"));
    }

    #[test]
    fn origem_de_pagina_desconhecida_nao_existe() {
        assert_eq!(bandeja_com(JATO).origem_da_pagina(u32::MAX), None);
    }

    /// Laser com o gesto certo: o verso só fecha se o maço for virado antes da leitura.
    fn laser_que_so_fecha_virando() -> Bandeja {
        let mut bandeja = bandeja_com(LASER);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4, 2], Giro::Nenhum));
        bandeja.recolocar(GESTO_LASER).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[1, 3, 5], Giro::Meia180));
        bandeja
    }

    #[test]
    fn fim_como_saiu_nao_deixa_virar_o_maco_passar_por_certo() {
        let mut bandeja = laser_que_so_fecha_virando();
        bandeja.encerrar(LeituraPedida::ComoSaiu);
        let tela = bandeja.tela(Idioma::PtBr);
        assert_eq!(tela.leitura, Some(LeituraTela::ComoSaiu), "{tela:?}");
        assert!(
            tela.folhas
                .iter()
                .any(|folha| folha.situacao == SituacaoDaFolha::Errada),
            "{tela:?}"
        );
        assert!(!tela.sem_instrucao, "{tela:?}");
    }

    #[test]
    fn fim_virando_confere_so_na_forma_de_virar() {
        let mut bandeja = laser_que_so_fecha_virando();
        bandeja.encerrar(LeituraPedida::Virando);
        let tela = bandeja.tela(Idioma::En);
        assert_eq!(tela.leitura, Some(LeituraTela::VirandoOMaco));
        assert_eq!((tela.corretas, tela.conferidas), (3, 3), "{tela:?}");
    }

    #[test]
    fn sem_instrucao_segue_o_modelo_da_suite_por_face_de_saida() {
        let mut laser = laser_que_so_fecha_virando();
        laser.encerrar(LeituraPedida::SemInstrucao);
        assert_eq!(
            laser.tela(Idioma::PtBr).leitura,
            Some(LeituraTela::VirandoOMaco)
        );
        let mut jato = jato_certo();
        jato.encerrar(LeituraPedida::SemInstrucao);
        assert_eq!(jato.tela(Idioma::PtBr).leitura, Some(LeituraTela::ComoSaiu));
    }

    #[test]
    fn sem_fim_a_tela_avisa_que_a_forma_foi_a_da_bandeja() {
        let tela = laser_que_so_fecha_virando().tela(Idioma::PtBr);
        assert_eq!(tela.leitura, Some(LeituraTela::VirandoOMaco));
        assert!(tela.sem_instrucao, "{tela:?}");
    }

    #[test]
    fn fim_que_chega_depois_da_segunda_passada_refaz_o_veredito() {
        let mut bandeja = laser_que_so_fecha_virando();
        let antes = bandeja.tela(Idioma::PtBr);
        assert!(antes.sem_instrucao, "{antes:?}");
        bandeja.encerrar(LeituraPedida::ComoSaiu);
        let depois = bandeja.tela(Idioma::PtBr);
        assert_eq!(depois.leitura, Some(LeituraTela::ComoSaiu), "{depois:?}");
        assert!(
            depois
                .folhas
                .iter()
                .any(|folha| folha.situacao == SituacaoDaFolha::Errada),
            "{depois:?}"
        );
    }

    #[test]
    fn limpar_esquece_o_fim_que_a_bandeja_recebeu() {
        let mut bandeja = laser_que_so_fecha_virando();
        bandeja.encerrar(LeituraPedida::ComoSaiu);
        bandeja.limpar();
        let tela = bandeja.tela(Idioma::PtBr);
        assert_eq!(tela.leitura, None);
        assert!(!tela.sem_instrucao);
    }

    #[test]
    fn nova_passada_inicial_esquece_o_fim_do_lote_anterior() {
        let mut bandeja = jato_certo();
        bandeja.encerrar(LeituraPedida::ComoSaiu);
        bandeja.receber("c.pdf".into(), numeradas(&[8], Giro::Nenhum));
        let tela = bandeja.tela(Idioma::PtBr);
        assert!(tela.sem_instrucao, "{tela:?}");
    }

    /// A tela do trabalho que nao e um documento inteiro: confere cada folha por si.
    fn so_erros_reais(tela: &BandejaTela) -> Vec<(u32, Vec<String>)> {
        tela.folhas
            .iter()
            .filter(|f| f.situacao == SituacaoDaFolha::Errada)
            .map(|f| (f.numero, f.motivos.clone()))
            .collect()
    }

    #[test]
    fn trecho_que_nao_comeca_na_pagina_1_confere_por_folha_sem_acusar_ordem() {
        // Paginas 3 a 6 de um documento maior, no jato: folhas (3,4) e (5,6).
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[5, 3], Giro::Meia180));
        let tela = bandeja.tela(Idioma::PtBr);
        assert!(tela.conferencia_por_folha, "{tela:?}");
        assert_eq!(so_erros_reais(&tela), vec![], "{tela:?}");
        assert_eq!((tela.corretas, tela.conferidas), (2, 2), "{tela:?}");
        assert!(tela.gerais.is_empty(), "{tela:?}");
    }

    #[test]
    fn copias_repetem_as_marcas_e_cada_folha_se_confere_sozinha() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[4, 2, 4, 2], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[3, 1, 3, 1], Giro::Meia180));
        let tela = bandeja.tela(Idioma::PtBr);
        assert!(tela.conferencia_por_folha, "{tela:?}");
        assert_eq!(so_erros_reais(&tela), vec![], "{tela:?}");
        assert_eq!(tela.corretas, 4, "{tela:?}");
    }

    #[test]
    fn papel_misto_em_dois_lotes_nao_acusa_ordem_nem_folha_a_menos() {
        // Lote A4 com as paginas 1 a 4; lote A5 com a pagina 5 sozinha (verso em branco).
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[4, 2], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[3, 1], Giro::Meia180));
        bandeja.receber("c.pdf".into(), numeradas(&[5], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber(
            "d.pdf".into(),
            vec![PaginaVista {
                giro: Giro::Meia180,
                ..em_branco(0)
            }],
        );
        let tela = bandeja.tela(Idioma::PtBr);
        assert!(tela.conferencia_por_folha, "{tela:?}");
        assert_eq!(so_erros_reais(&tela), vec![], "{tela:?}");
        assert_eq!(tela.corretas, 3, "{tela:?}");
        assert!(tela.gerais.is_empty(), "{tela:?}");
    }

    #[test]
    fn erro_real_numa_folha_continua_errada_no_trecho() {
        // Borda errada na recolocacao: os versos saem de cabeca para baixo.
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4], Giro::Nenhum));
        bandeja
            .recolocar(Gesto {
                borda_que_entra: BordaQueEntra::Topo,
                ..GESTO_JATO
            })
            .unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[5, 3], Giro::Meia180));
        let tela = bandeja.tela(Idioma::PtBr);
        assert!(tela.conferencia_por_folha, "{tela:?}");
        assert!(!so_erros_reais(&tela).is_empty(), "{tela:?}");
    }

    #[test]
    fn verso_que_nao_segue_a_frente_na_mesma_folha_e_erro_mesmo_por_folha() {
        // A folha com a 3 na frente e a 6 no verso: o par nao bate.
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[5, 3], Giro::Meia180));
        let certo = bandeja.tela(Idioma::PtBr);
        assert_eq!(so_erros_reais(&certo), vec![], "{certo:?}");
        let mut trocada = bandeja_com(JATO);
        trocada.receber("a.pdf".into(), numeradas(&[4, 6], Giro::Nenhum));
        trocada.recolocar(GESTO_JATO).unwrap();
        trocada.receber("b.pdf".into(), numeradas(&[5, 3], Giro::Meia180));
        let tela = trocada.tela(Idioma::PtBr);
        assert!(!so_erros_reais(&tela).is_empty(), "{tela:?}");
    }

    #[test]
    fn documento_inteiro_de_um_lote_continua_com_a_conferencia_do_maco() {
        let tela = jato_certo().tela(Idioma::PtBr);
        assert!(!tela.conferencia_por_folha, "{tela:?}");
    }

    #[test]
    fn com_a_instrucao_do_fim_o_veredito_segue_como_sempre() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber("a.pdf".into(), numeradas(&[6, 4], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[5, 3], Giro::Meia180));
        bandeja.encerrar(LeituraPedida::ComoSaiu);
        let tela = bandeja.tela(Idioma::PtBr);
        assert!(!tela.conferencia_por_folha, "{tela:?}");
        assert!(!so_erros_reais(&tela).is_empty(), "{tela:?}");
    }

    #[test]
    fn frente_que_chega_com_a_bandeja_aguardando_verso_abre_lote_novo() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber_frente("a.pdf".into(), numeradas(&[4, 2], Giro::Nenhum));
        bandeja.receber_frente("b.pdf".into(), numeradas(&[8, 6], Giro::Nenhum));
        let tela = bandeja.tela(Idioma::PtBr);
        // As duas primeiras folhas ficaram como lote proprio; as outras duas esperam o gesto.
        assert_eq!(tela.etapa, EtapaTela::AguardandoGesto, "{tela:?}");
        assert_eq!(tela.folhas.len(), 4, "{tela:?}");
        let andamento = tela
            .folhas
            .iter()
            .filter(|f| f.situacao == SituacaoDaFolha::EmAndamento)
            .count();
        assert_eq!(andamento, 2, "{tela:?}");
        // Nada virou verso: o gesto agora serve ao lote novo.
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("c.pdf".into(), numeradas(&[7, 5], Giro::Meia180));
        let fim = bandeja.tela(Idioma::PtBr);
        assert_eq!(fim.etapa, EtapaTela::Conferida, "{fim:?}");
        assert!(fim.conferencia_por_folha, "{fim:?}");
    }

    #[test]
    fn verso_do_plots_continua_sendo_a_segunda_passada_do_lote() {
        let mut bandeja = bandeja_com(JATO);
        bandeja.receber_frente("a.pdf".into(), numeradas(&[6, 4, 2], Giro::Nenhum));
        bandeja.recolocar(GESTO_JATO).unwrap();
        bandeja.receber("b.pdf".into(), numeradas(&[5, 3, 1], Giro::Meia180));
        let tela = bandeja.tela(Idioma::PtBr);
        assert_eq!((tela.corretas, tela.conferidas), (3, 3), "{tela:?}");
    }
}
