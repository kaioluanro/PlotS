//! Um trabalho por impressora, cada um em uma thread propria, fora da thread da interface.
//!
//! Cada thread conduz um documento em uma impressora e publica o cartao a cada mudanca. A interface
//! manda comandos por um canal; impressoras diferentes nunca esperam umas pelas outras. Nada aqui
//! tira um trabalho da fila sem um pedido do usuario.

use crate::documento;
use crate::impressoras;
use crate::leitura_pdf;
use crate::tempo_de_impressao::{Cronometro, Medida, Relogio};
use plots_core::distribuicao::ModoDistribuicao;
use plots_core::{
    plano_automatico_copias_do_intervalo, plano_copias_do_intervalo, DimensoesPagina, FormatoPapel,
    IntervaloPaginas, LotePapel, ModeloImpressora, OrdemPassadas, Pagina, Papel, PerfilImpressora,
    RepartoAutomatico,
};
use plots_perfis::historico::ResultadoDoEnvio;
use plots_windows::plots_print::fluxo::{
    self, CondicaoDoAviso, Condutor, DescreveFalha, Espera, Instrucao, Lado, LoteDoFluxo, Passo,
    SimGuiado, Virada,
};
use plots_windows::plots_print::job::{Alcance, Backend, ErroJob, EstadoJob};
use plots_windows::plots_print::opcoes::{
    compensar_driver, marcar_lotes, DriverDeTeste, IdiomaDaMarca,
};
use plots_windows::plots_print::{ErroSimulacao, Mecanica};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// A simulacao nao tem fila: o intervalo so deixa o estado aparecer antes da confirmacao.
pub(crate) const ESPERA_SIMULADA: Espera = Espera {
    intervalo: Duration::from_millis(400),
    consultas: 1_800,
};
/// Nome do evento que leva o cartao a interface.
pub const EVENTO_DO_CARTAO: &str = "trabalho";

/// Motivo de um aviso ou erro. `texto` vai para quem le portugues; a interface traduz o `codigo`
/// nos outros idiomas, porque as mensagens de envio so existem em portugues.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Motivo {
    pub codigo: &'static str,
    pub texto: String,
}

impl Motivo {
    pub(crate) fn novo(codigo: &'static str, texto: impl Into<String>) -> Self {
        Self {
            codigo,
            texto: texto.into(),
        }
    }
}

/// Erro de backend em forma de motivo.
pub trait ParaMotivo {
    fn para_motivo(&self) -> Motivo;
}

/// O driver nao ficou com as duas faces: nada saiu, e o caminho e escolher um modelo manual.
pub const MOTIVO_DUAS_FACES_RECUSADAS: &str = "duas_faces_recusadas";

impl ParaMotivo for ErroSimulacao {
    fn para_motivo(&self) -> Motivo {
        let codigo = match self {
            Self::DuasFacesRecusadas => MOTIVO_DUAS_FACES_RECUSADAS,
            Self::EstadoInvalido { .. } | Self::QuantidadeFolhas { .. } => "sistema",
        };
        Motivo::novo(codigo, self.to_string())
    }
}

impl ParaMotivo for plots_windows::ErroEnvio {
    fn para_motivo(&self) -> Motivo {
        use plots_windows::ErroEnvio as E;
        let codigo = match self {
            E::BibliotecaAusente { .. }
            | E::BibliotecaCaminhoRelativo(_)
            | E::BibliotecaInvalida(_) => "biblioteca",
            E::Pdf(_) | E::PassadaVazia | E::PaginaInexistente(_) | E::TamanhoInvalido => "pdf",
            E::Rasterizacao(_) | E::MemoriaExcedida { .. } | E::PaginaForaDaArea => "desenho",
            E::PapelGrandeDemais { .. } | E::PapelNaoAceito { .. } => "papel",
            E::SimplexNaoAceito
            | E::ValorNaoAceito { .. }
            | E::ValorNaoConfirmado { .. }
            | E::SemSuporteABitmap => "driver",
            E::DuasFacesNaoAceitas => MOTIVO_DUAS_FACES_RECUSADAS,
            E::PastaDeSaidaInvalida(_) | E::ArquivoJaExiste(_) | E::ArquivoNaoGerado(_) => {
                "arquivo_saida"
            }
            E::FilaSaiuDaLista => "fila_removida",
            E::NaoSuportado
            | E::Sistema { .. }
            | E::Resposta(_)
            | E::TextoInvalido
            | E::SemTrabalho => "sistema",
        };
        Motivo::novo(codigo, self.to_string())
    }
}

/// A condicao que a fila distinguiu vira um codigo proprio: a interface decide a orientacao por ele.
fn codigo_do_aviso(condicao: Option<CondicaoDoAviso>) -> &'static str {
    match condicao {
        Some(CondicaoDoAviso::SemPapel) => "sem_papel",
        Some(CondicaoDoAviso::Atolado) => "atolado",
        Some(CondicaoDoAviso::SemResposta) => "sem_resposta",
        None => "fila",
    }
}

pub(crate) fn motivo_do_job<E: ParaMotivo>(erro: &ErroJob<E>) -> Motivo {
    match erro {
        ErroJob::Backend(erro) => erro.para_motivo(),
        ErroJob::EnvioFalhou => Motivo::novo("fila", "o spooler informou uma falha"),
        ErroJob::SemLotes => Motivo::novo("pdf", "o documento não tem lotes para enviar"),
        ErroJob::EstadoInvalido(estado) => Motivo::novo(
            "sistema",
            format!("operação fora de ordem no estado {estado:?}"),
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PapelDoCartao {
    pub formato: &'static str,
    pub largura_mm: f64,
    pub altura_mm: f64,
}

pub fn papel_do_cartao(papel: &Papel) -> PapelDoCartao {
    PapelDoCartao {
        formato: match papel.formato {
            FormatoPapel::A4 => "a4",
            FormatoPapel::A5 => "a5",
            FormatoPapel::Carta => "carta",
            FormatoPapel::Oficio => "oficio",
            FormatoPapel::Legal => "legal",
            FormatoPapel::Personalizado => "outro",
        },
        largura_mm: papel.largura_mm,
        altura_mm: papel.altura_mm,
    }
}

/// O que dizer ao recolocar. Sem modelo, o perfil nao afirma lado: vale o gesto da calibracao.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recolocar {
    pub lado: Option<&'static str>,
    pub virada: Option<&'static str>,
    pub texto_do_perfil: Option<String>,
}

impl Recolocar {
    fn da_instrucao(instrucao: Instrucao) -> Self {
        Self::do_lado_e_virada(instrucao.lado, instrucao.virada)
    }

    fn do_lado_e_virada(lado: Lado, virada: Virada) -> Self {
        Self {
            lado: Some(match lado {
                Lado::Cima => "cima",
                Lado::Baixo => "baixo",
            }),
            virada: Some(match virada {
                Virada::PelaBordaDeCima => "borda_de_cima",
                Virada::PelaLateral => "lateral",
                Virada::NaoVirar => "nao_virar",
                Virada::Girar => "girar",
            }),
            texto_do_perfil: None,
        }
    }

    /// O gesto da calibracao guiada: a virada do perfil, e o lado em que a face impressa entra.
    pub(crate) fn calibrado(perfil: PerfilImpressora, entra_para_cima: bool) -> Self {
        let lado = if entra_para_cima {
            Lado::Cima
        } else {
            Lado::Baixo
        };
        let virada = match perfil.gesto_recolocar.virada {
            plots_core::ViradaMaco::NaoVirar => Virada::NaoVirar,
            plots_core::ViradaMaco::PelaLateral => Virada::PelaLateral,
            plots_core::ViradaMaco::PelaBordaDeCima => Virada::PelaBordaDeCima,
        };
        Self::do_lado_e_virada(lado, virada)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FaceDaSequencia {
    pub numero: Option<usize>,
    pub girada: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sequencia {
    pub frente: Vec<FaceDaSequencia>,
    pub verso: Vec<FaceDaSequencia>,
    /// O que sai escrito nas folhas, na ordem das passadas. Vazio sem a folha de controle.
    pub marcas: Vec<String>,
}

fn faces(paginas: &[Pagina]) -> Vec<FaceDaSequencia> {
    paginas
        .iter()
        .map(|pagina| FaceDaSequencia {
            numero: pagina.numero,
            girada: pagina.girar_180,
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Resultado {
    pub corretas: usize,
    pub folhas: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PassadaDoCartao {
    Frente,
    Verso,
    /// O lote inteiro em um trabalho, com a impressora imprimindo as duas faces sozinha.
    DuasFaces,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum EstadoCartao {
    /// Lendo o PDF e abrindo a fila; nada foi enviado ainda.
    Preparando,
    Imprimindo {
        passada: PassadaDoCartao,
    },
    /// A fila confirmou a frente: so aqui o verso libera.
    Aguardando,
    /// Todas as frentes pedidas sairam. O trabalho acabou: o verso e outro pedido, para quando quiser.
    FrentesProntas,
    /// O verso terminou e ainda ha lotes.
    ProximoLote,
    Aviso {
        passada: PassadaDoCartao,
        motivo: Motivo,
        /// O que o proprio driver disse: a interface mostra em linha propria, sem decidir nada por ela.
        #[serde(rename = "mensagemDoDriver")]
        mensagem_do_driver: Option<String>,
        respondido: bool,
    },
    Erro {
        motivo: Motivo,
    },
    Concluido,
    /// O pedido de cancelar chegou e a fila ainda tira o trabalho: pode levar segundos.
    Cancelando,
    Cancelado,
}

/// O aviso que o cartao esta mostrando.
#[derive(Debug, Clone)]
struct AvisoAberto {
    motivo: Motivo,
    mensagem_do_driver: Option<String>,
    respondido: bool,
}

/// Tudo o que a interface mostra de uma impressora.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cartao {
    pub aba: u64,
    pub impressora: String,
    pub nome: String,
    pub simulada: bool,
    pub lote: usize,
    pub total: usize,
    pub papel: Option<PapelDoCartao>,
    pub estado: EstadoCartao,
    pub recolocar: Option<Recolocar>,
    pub troca_de_papel: Option<PapelDoCartao>,
    pub resultado: Option<Resultado>,
    pub sequencia: Option<Sequencia>,
    pub vira_para_ler: bool,
    /// O driver nao aceitou o ajuste salvo: este trabalho saiu com o padrao da impressora.
    pub ajuste_recusado: bool,
    /// O lote atual sai sozinho nas duas faces: sem gesto e sem verso a pedir.
    pub lote_automatico: bool,
    /// O lote que acabou de terminar saiu sozinho nas duas faces.
    pub anterior_automatico: bool,
    /// Algum lote deste documento sai sozinho nas duas faces nesta impressora.
    pub tem_automatico: bool,
    /// Folhas do lote atual: e o que sai de novo ao reimprimir.
    pub folhas_do_lote: usize,
    /// As paginas do lote atual: e por elas que a pessoa acha o maco.
    pub faixa_do_lote: Option<FaixaDoLote>,
    /// O que este trabalho imprime: `so_frentes`, `so_versos` ou `frente_e_verso`.
    pub alcance: &'static str,
    /// A cópia que o lote atual imprime, a partir de 1. `None` quando o lote mistura cópias.
    pub copia: Option<usize>,
    /// Quantas cópias o trabalho inteiro imprime.
    pub copias: usize,
}

/// A cópia que o lote inteiro imprime, a partir de 1; `None` quando o lote mistura cópias.
pub fn copia_unica(paginas: &[Pagina]) -> Option<usize> {
    let primeira = paginas.first()?.copia;
    paginas
        .iter()
        .all(|pagina| pagina.copia == primeira)
        .then(|| primeira.saturating_add(1))
}

/// Quantas cópias o trabalho imprime: a maior cópia de todos os lotes, mais uma.
pub fn copias_do_trabalho(lotes: &[LoteDoFluxo]) -> usize {
    lotes
        .iter()
        .flat_map(|item| item.lote.paginas.iter().map(|pagina| pagina.copia))
        .max()
        .map_or(1, |maior| maior.saturating_add(1))
}

#[derive(Debug, Clone)]
pub struct Identidade {
    pub aba: u64,
    pub impressora: String,
    pub nome: String,
    pub simulada: bool,
}

impl Identidade {
    fn cartao(&self, estado: EstadoCartao) -> Cartao {
        Cartao {
            aba: self.aba,
            impressora: self.impressora.clone(),
            nome: self.nome.clone(),
            simulada: self.simulada,
            lote: 0,
            total: 0,
            papel: None,
            estado,
            recolocar: None,
            troca_de_papel: None,
            resultado: None,
            sequencia: None,
            vira_para_ler: false,
            ajuste_recusado: false,
            lote_automatico: false,
            anterior_automatico: false,
            tem_automatico: false,
            folhas_do_lote: 0,
            faixa_do_lote: None,
            alcance: "frente_e_verso",
            copia: None,
            copias: 1,
        }
    }
}

/// Pedido da interface a um trabalho em andamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comando {
    EnviarFrente,
    EnviarVerso,
    Reimprimir,
    Cancelar,
    ContinuarEsperando,
}

/// O que a thread recebe: um comando, ou a ordem de parar de acompanhar sem tocar na fila.
enum Recado {
    Comando(Comando),
    Soltar,
}

pub type Emissor = Arc<dyn Fn(Cartao) + Send + Sync>;

/// O que acompanha o condutor durante um trabalho, alem do que ele sabe sozinho.
pub struct Preparo<B: Backend> {
    pub condutor: Condutor<B>,
    pub recolocar: Recolocar,
    pub vira_para_ler: bool,
}

struct Andamento<'a, B: Backend> {
    condutor: Condutor<B>,
    identidade: &'a Identidade,
    emitir: &'a Emissor,
    recolocar: Recolocar,
    vira_para_ler: bool,
    resultado: Option<Resultado>,
    aviso: Option<AvisoAberto>,
    /// Sem consultas ate o usuario decidir: a fila nao responde ou nao confirmou a tempo.
    parado: bool,
    passada: PassadaDoCartao,
    ajuste_recusado: bool,
    cronometro: Cronometro,
    relogio: &'a dyn Relogio,
    lotes_feitos: u64,
    /// Como terminou, quando terminou: sem isto, o trabalho só parou de ser acompanhado.
    fim: Option<ResultadoDoEnvio>,
    com_erro: bool,
    /// Verdadeiro enquanto o cartao esta em erro: o trabalho so espera Reimprimir ou Cancelar e nao
    /// pode trancar o envio de um novo.
    substituivel: Arc<AtomicBool>,
}

impl<B> Andamento<'_, B>
where
    B: Backend + DescreveFalha,
    B::Erro: ParaMotivo + Display,
{
    fn publicar(&self, estado: EstadoCartao) {
        self.substituivel.store(
            matches!(estado, EstadoCartao::Erro { .. }),
            Ordering::SeqCst,
        );
        let mut cartao = self.identidade.cartao(estado);
        cartao.lote = self.condutor.numero();
        cartao.total = self.condutor.total();
        cartao.papel = self
            .condutor
            .lote()
            .map(|item| papel_do_cartao(&item.papel));
        cartao.recolocar = Some(self.recolocar.clone());
        cartao.vira_para_ler = self.vira_para_ler;
        cartao.resultado = self.resultado;
        cartao.ajuste_recusado = self.ajuste_recusado;
        cartao.lote_automatico = self.condutor.lote_em_duas_faces();
        cartao.anterior_automatico = self.condutor.terminado_em_duas_faces();
        cartao.tem_automatico = self.condutor.tem_lote_em_duas_faces();
        cartao.folhas_do_lote = self
            .condutor
            .lote()
            .map_or(0, |item| item.lote.paginas.len().div_ceil(2));
        cartao.faixa_do_lote = self.condutor.lote().and_then(|item| {
            let numeros = item.lote.paginas.iter().filter_map(|pagina| pagina.numero);
            Some(FaixaDoLote {
                lote: self.condutor.numero(),
                inicio: numeros.clone().min()?,
                fim: numeros.max()?,
            })
        });
        cartao.alcance = match self.condutor.alcance() {
            Alcance::FrenteEVerso => "frente_e_verso",
            Alcance::SoFrentes => "so_frentes",
            Alcance::SoVersos => "so_versos",
        };
        cartao.copia = self
            .condutor
            .lote()
            .and_then(|item| copia_unica(&item.lote.paginas));
        cartao.copias = copias_do_trabalho(self.condutor.lotes());
        cartao.troca_de_papel = self
            .condutor
            .troca_de_papel()
            .map(|papel| papel_do_cartao(&papel));
        if self.identidade.simulada {
            cartao.sequencia = self.condutor.lote().map(|item| Sequencia {
                frente: faces(&item.lote.passada1),
                verso: faces(&item.lote.passada2),
                marcas: item
                    .lote
                    .passada1
                    .iter()
                    .chain(&item.lote.passada2)
                    .filter_map(|face| item.marcas.get(&face.identidade()).cloned())
                    .collect(),
            });
        }
        (self.emitir)(cartao);
    }

    fn publicar_erro(&mut self, motivo: Motivo) {
        self.com_erro = true;
        self.cronometro.pausar(self.relogio.agora_ms());
        self.publicar(EstadoCartao::Erro { motivo });
    }

    /// O lote terminou na fila: as folhas dele contam, e o tempo das duas passadas também.
    fn fechar_lote(&mut self) {
        let folhas = self.condutor.lote_terminado().map_or(0, |item| {
            u64::try_from(item.lote.paginas.len().div_ceil(2)).unwrap_or(u64::MAX)
        });
        let automatico = self.condutor.terminado_em_duas_faces();
        self.cronometro
            .fechar_lote(self.relogio.agora_ms(), folhas, automatico);
        self.lotes_feitos = self.lotes_feitos.saturating_add(1);
    }

    fn desfecho(&self) -> Desfecho {
        let lotes = self.condutor.lotes();
        let paginas = lotes
            .iter()
            .flat_map(|item| &item.lote.paginas)
            .filter(|pagina| pagina.numero.is_some())
            .count();
        Desfecho {
            resultado: self.fim.unwrap_or(if self.com_erro {
                ResultadoDoEnvio::ComErro
            } else {
                ResultadoDoEnvio::Incompleto
            }),
            lotes: u64::try_from(lotes.len()).unwrap_or(u64::MAX),
            lotes_feitos: self.lotes_feitos,
            paginas: u64::try_from(paginas).unwrap_or(u64::MAX),
            medida: self.cronometro.medida(),
            medida_automatica: self.cronometro.medida_automatica(),
        }
    }

    fn publicar_aviso(&mut self) {
        if let Some(aviso) = self.aviso.clone() {
            self.publicar(EstadoCartao::Aviso {
                passada: self.passada,
                motivo: aviso.motivo,
                mensagem_do_driver: aviso.mensagem_do_driver,
                respondido: aviso.respondido,
            });
        }
    }

    /// A fila recusou o cancelamento: o trabalho segue la, e o cartao manda conferir o Windows.
    fn falhou_ao_cancelar(&mut self, erro: &ErroJob<B::Erro>) {
        self.parado = true;
        let detalhe = motivo_do_job(erro).texto;
        self.publicar_erro(Motivo::novo(
            "nao_cancelou",
            format!("Não foi possível cancelar o trabalho ({detalhe}). Confira a fila do Windows."),
        ));
    }

    /// Como o lote atual sai. O lote em duas faces nao tem frente nem verso separados.
    fn primeira_passada(&self) -> PassadaDoCartao {
        if self.condutor.alcance() == Alcance::SoVersos {
            PassadaDoCartao::Verso
        } else if self.condutor.lote_em_duas_faces() {
            PassadaDoCartao::DuasFaces
        } else {
            PassadaDoCartao::Frente
        }
    }

    /// Mostra a falha do envio. Se o driver recusou as duas faces, nada foi para a fila e nada vai
    /// mudar sem outro modelo: a thread acaba, e a impressora fica livre para a pessoa enviar de novo.
    fn falhou_ao_enviar(&mut self, erro: &ErroJob<B::Erro>) -> Continua {
        let motivo = motivo_do_job(erro);
        let acabou = motivo.codigo == MOTIVO_DUAS_FACES_RECUSADAS;
        self.publicar_erro(motivo);
        if acabou {
            Continua::Nao
        } else {
            Continua::Sim
        }
    }

    fn enviar(&mut self, passada: PassadaDoCartao) -> Continua {
        self.aviso = None;
        self.parado = false;
        self.com_erro = false;
        self.passada = passada;
        self.cronometro.iniciar_passada(self.relogio.agora_ms());
        // O estado aparece antes: desenhar as paginas de uma passada pode levar minutos.
        self.publicar(EstadoCartao::Imprimindo { passada });
        let enviado = match passada {
            PassadaDoCartao::Frente | PassadaDoCartao::DuasFaces => self.condutor.enviar_frente(),
            PassadaDoCartao::Verso => self.condutor.enviar_verso(),
        };
        let recusou =
            !self.identidade.simulada && crate::preferencias::tomar_recusa(&self.identidade.nome);
        self.ajuste_recusado |= recusou;
        match enviado {
            Err(erro) => return self.falhou_ao_enviar(&erro),
            // A recusa aparece logo: o proximo estado pode demorar a mudar.
            Ok(_) if recusou => self.publicar(EstadoCartao::Imprimindo { passada }),
            Ok(_) => {}
        }
        Continua::Sim
    }

    fn somar_resultado(&mut self) {
        let Some(item) = self.condutor.lote_terminado() else {
            return;
        };
        // So a frente saiu: ainda nao ha folha pronta para conferir.
        if self.condutor.alcance() == Alcance::SoFrentes && !item.lote.duas_faces {
            return;
        }
        let Some((corretas, folhas)) = self
            .condutor
            .backend()
            .folhas_conferidas(&item.lote.paginas)
        else {
            return;
        };
        let antes = self.resultado.unwrap_or(Resultado {
            corretas: 0,
            folhas: 0,
        });
        self.resultado = Some(Resultado {
            corretas: antes.corretas.saturating_add(corretas),
            folhas: antes.folhas.saturating_add(folhas),
        });
    }

    fn cancelar(&mut self) -> Continua {
        let tem_passada_na_fila = matches!(
            self.condutor.estado(),
            EstadoJob::FrenteEnviada
                | EstadoJob::VersoEnviado
                | EstadoJob::DuasFacesEnviadas
                | EstadoJob::Erro
        );
        if tem_passada_na_fila {
            // Tirar o trabalho da fila pode levar segundos: a pessoa ve que o pedido foi aceito.
            self.publicar(EstadoCartao::Cancelando);
            if let Err(erro) = self.condutor.cancelar() {
                self.falhou_ao_cancelar(&erro);
                return Continua::Sim;
            }
        }
        self.cronometro.pausar(self.relogio.agora_ms());
        self.fim = Some(ResultadoDoEnvio::Cancelado);
        self.publicar(EstadoCartao::Cancelado);
        Continua::Nao
    }

    /// Depois de tirar o trabalho da fila, refaz so o que este trabalho imprime.
    fn enviar_a_passada_de_novo(&mut self) -> Result<(), ErroJob<B::Erro>> {
        if self.condutor.alcance() == Alcance::SoVersos {
            self.condutor.enviar_verso()
        } else {
            self.condutor.enviar_frente()
        }
    }

    fn atender(&mut self, comando: Comando) -> Continua {
        let estado = self.condutor.estado();
        match comando {
            Comando::EnviarFrente if estado == EstadoJob::Pronto => {
                return self.enviar(self.primeira_passada());
            }
            Comando::EnviarVerso if estado == EstadoJob::AguardandoRecolocacao => {
                return self.enviar(PassadaDoCartao::Verso);
            }
            // Com aviso, o trabalho ainda esta na fila: refazer o lote comeca por tira-lo de la.
            Comando::Reimprimir
                if estado == EstadoJob::Erro || self.parado || self.aviso.is_some() =>
            {
                self.aviso = None;
                self.parado = false;
                self.com_erro = false;
                self.passada = self.primeira_passada();
                self.cronometro.descartar_lote();
                self.publicar(EstadoCartao::Imprimindo {
                    passada: self.passada,
                });
                // Antes de mandar o lote de novo, o trabalho anterior sai da fila; se a fila recusar, nada e enviado.
                self.cronometro.iniciar_passada(self.relogio.agora_ms());
                if let Err(erro) = self.condutor.cancelar() {
                    self.falhou_ao_cancelar(&erro);
                } else if let Err(erro) = self.enviar_a_passada_de_novo() {
                    self.parado = true;
                    return self.falhou_ao_enviar(&erro);
                }
            }
            Comando::Cancelar => return self.cancelar(),
            Comando::ContinuarEsperando => {
                self.condutor.continuar_esperando();
                self.parado = false;
                if let Some(aviso) = self.aviso.as_mut() {
                    aviso.respondido = true;
                }
                self.publicar_aviso();
            }
            // Fora de ordem nada e enviado: o verso so sai com a frente confirmada pelo trabalho.
            Comando::EnviarFrente | Comando::EnviarVerso | Comando::Reimprimir => {}
        }
        Continua::Sim
    }

    fn consultar(&mut self) -> Continua {
        let passo = self.condutor.consultar();
        let agora = self.relogio.agora_ms();
        match passo {
            Passo::EmAndamento => {
                self.cronometro.retomar(agora);
                if self.aviso.take().is_some() {
                    self.publicar(EstadoCartao::Imprimindo {
                        passada: self.passada,
                    });
                }
            }
            Passo::Aviso {
                motivo,
                condicao,
                mensagem_do_driver,
                novo,
            } => {
                self.cronometro.pausar(agora);
                if novo {
                    self.aviso = Some(AvisoAberto {
                        motivo: Motivo::novo(codigo_do_aviso(condicao), motivo),
                        mensagem_do_driver,
                        respondido: false,
                    });
                    self.publicar_aviso();
                } else if let Some(aviso) = self
                    .aviso
                    .as_mut()
                    .filter(|aviso| aviso.mensagem_do_driver != mensagem_do_driver)
                {
                    // So a mensagem mudou: o cartao atualiza a linha, sem desfazer a resposta.
                    aviso.mensagem_do_driver = mensagem_do_driver;
                    self.publicar_aviso();
                }
            }
            Passo::FrenteConfirmada => {
                self.aviso = None;
                self.cronometro.fechar_passada(agora);
                self.publicar(EstadoCartao::Aguardando);
            }
            Passo::LoteConcluido => {
                self.aviso = None;
                self.fechar_lote();
                self.somar_resultado();
                // Sem gesto e sem troca de papel, nao ha o que esperar de ninguem.
                if self.condutor.proximo_segue_sozinho() {
                    return self.enviar(PassadaDoCartao::DuasFaces);
                }
                self.publicar(if self.condutor.alcance() == Alcance::SoVersos {
                    EstadoCartao::Aguardando
                } else {
                    EstadoCartao::ProximoLote
                });
            }
            Passo::DocumentoConcluido => {
                self.fechar_lote();
                self.fim = Some(ResultadoDoEnvio::Concluido);
                self.somar_resultado();
                self.publicar(
                    if self.condutor.alcance() == Alcance::SoFrentes
                        && self.condutor.tem_lote_manual()
                    {
                        EstadoCartao::FrentesProntas
                    } else {
                        EstadoCartao::Concluido
                    },
                );
                return Continua::Nao;
            }
            Passo::Falhou { motivo } => {
                self.aviso = None;
                let motivo = motivo.map_or_else(
                    || motivo_do_job::<B::Erro>(&ErroJob::EnvioFalhou),
                    |texto| Motivo::novo("fila", texto),
                );
                self.publicar_erro(motivo);
            }
            Passo::SemAcompanhamento { erro, saiu_da_fila } => {
                self.aviso = None;
                self.parado = true;
                let codigo = if saiu_da_fila {
                    "sem_confirmacao"
                } else {
                    "sem_acompanhamento"
                };
                self.publicar_erro(Motivo::novo(codigo, erro.to_string()));
            }
            Passo::TempoEsgotado => {
                self.parado = true;
                self.cronometro.pausar(agora);
                let motivo = Motivo::novo(
                    "sem_prazo",
                    "a fila não confirmou a tempo; o trabalho continua na fila",
                );
                self.aviso = Some(AvisoAberto {
                    motivo,
                    mensagem_do_driver: None,
                    respondido: false,
                });
                self.publicar_aviso();
            }
            Passo::ForaDeOrdem(erro) => {
                self.parado = true;
                self.publicar_erro(motivo_do_job(&erro));
            }
        }
        Continua::Sim
    }

    fn consulta_a_fila(&self) -> bool {
        !self.parado
            && matches!(
                self.condutor.estado(),
                EstadoJob::FrenteEnviada | EstadoJob::VersoEnviado | EstadoJob::DuasFacesEnviadas
            )
    }
}

#[derive(PartialEq, Eq)]
enum Continua {
    Sim,
    Nao,
}

/// Envia a primeira frente e conduz o documento ate o fim, o cancelamento ou a ordem de soltar.
fn conduzir<B>(
    preparo: Preparo<B>,
    identidade: &Identidade,
    recados: &Receiver<Recado>,
    emitir: &Emissor,
    relogio: &dyn Relogio,
    substituivel: Arc<AtomicBool>,
) -> Desfecho
where
    B: Backend + DescreveFalha,
    B::Erro: ParaMotivo + Display,
{
    let mut andamento = Andamento {
        condutor: preparo.condutor,
        identidade,
        emitir,
        recolocar: preparo.recolocar,
        vira_para_ler: preparo.vira_para_ler,
        resultado: None,
        aviso: None,
        parado: false,
        passada: PassadaDoCartao::Frente,
        ajuste_recusado: false,
        cronometro: Cronometro::default(),
        relogio,
        lotes_feitos: 0,
        fim: None,
        com_erro: false,
        substituivel,
    };
    acompanhar(&mut andamento, recados);
    andamento.desfecho()
}

fn acompanhar<B>(andamento: &mut Andamento<'_, B>, recados: &Receiver<Recado>)
where
    B: Backend + DescreveFalha,
    B::Erro: ParaMotivo + Display,
{
    // Os recados chegados durante a preparacao valem antes do primeiro envio: soltar a aba ou
    // cancelar nesse intervalo nao pode mandar a frente para a fila.
    loop {
        match recados.try_recv() {
            Ok(Recado::Soltar) | Err(TryRecvError::Disconnected) => return,
            Ok(Recado::Comando(Comando::Cancelar)) => {
                andamento.fim = Some(ResultadoDoEnvio::Cancelado);
                andamento.publicar(EstadoCartao::Cancelado);
                return;
            }
            Ok(Recado::Comando(_)) => {}
            Err(TryRecvError::Empty) => break,
        }
    }
    // So de versos, sem nenhum lote que peca o gesto: nao ha o que imprimir nesta impressora.
    if andamento.condutor.estado() == EstadoJob::Concluido {
        andamento.fim = Some(ResultadoDoEnvio::Concluido);
        andamento.publicar(EstadoCartao::Concluido);
        return;
    }
    if andamento.enviar(andamento.primeira_passada()) == Continua::Nao {
        return;
    }
    loop {
        let recado = if andamento.consulta_a_fila() {
            match recados.recv_timeout(andamento.condutor.intervalo()) {
                Ok(recado) => Some(recado),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else {
            match recados.recv() {
                Ok(recado) => Some(recado),
                Err(_) => return,
            }
        };
        let seguir = match recado {
            Some(Recado::Soltar) => return,
            Some(Recado::Comando(comando)) => andamento.atender(comando),
            None => andamento.consultar(),
        };
        if seguir == Continua::Nao {
            return;
        }
    }
}

/// Avisa o fim da thread, de qualquer forma que ele venha. Uma thread interrompida vira erro no
/// cartao em vez de deixar o usuario esperando.
struct GuardaDaThread {
    identidade: Identidade,
    emitir: Emissor,
    ativo: Arc<AtomicBool>,
    fim: Option<FimDoTrabalho>,
    desfecho: Option<Desfecho>,
}

impl Drop for GuardaDaThread {
    fn drop(&mut self) {
        // A impressora so fica livre depois: quem espera por ela encontra o historico ja escrito.
        if let Some(fim) = self.fim.take() {
            fim(self.desfecho.take().unwrap_or(Desfecho::sem_envio()));
        }
        self.ativo.store(false, Ordering::SeqCst);
        if std::thread::panicking() {
            (self.emitir)(self.identidade.cartao(EstadoCartao::Erro {
                motivo: motivo_de_interrupcao(),
            }));
        }
    }
}

/// Como um trabalho acabou nesta impressora, para o historico.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Desfecho {
    pub resultado: ResultadoDoEnvio,
    pub lotes: u64,
    pub lotes_feitos: u64,
    pub paginas: u64,
    pub medida: Medida,
    /// O que saiu em duas faces fica a parte: e outra velocidade, mesmo na mesma impressora.
    pub medida_automatica: Medida,
}

impl Desfecho {
    /// Nada chegou a ser enviado, ou a thread parou antes de dizer como foi.
    fn sem_envio() -> Self {
        Self {
            resultado: ResultadoDoEnvio::ComErro,
            lotes: 0,
            lotes_feitos: 0,
            paginas: 0,
            medida: Medida::default(),
            medida_automatica: Medida::default(),
        }
    }
}

/// Chamado uma vez, ao fim da thread de cada trabalho, seja qual for o fim.
pub type FimDoTrabalho = Box<dyn FnOnce(Desfecho) + Send>;

fn motivo_de_interrupcao() -> Motivo {
    Motivo::novo(
        "interrompido",
        "o acompanhamento deste trabalho foi interrompido; veja a fila do Windows antes de enviar de novo",
    )
}

struct Entrada {
    aba: u64,
    recados: Sender<Recado>,
    ativo: Arc<AtomicBool>,
    substituivel: Arc<AtomicBool>,
}

/// Os trabalhos em andamento, um por impressora.
#[derive(Default)]
pub struct Gerente {
    trabalhos: Mutex<HashMap<String, Entrada>>,
}

impl Gerente {
    /// Inicia o trabalho em uma thread propria. Devolve `false`, sem mudar nada, se a impressora
    /// ja tem um trabalho em andamento.
    #[cfg(test)]
    pub fn iniciar<B, F>(&self, identidade: Identidade, emitir: Emissor, preparar: F) -> bool
    where
        B: Backend + DescreveFalha,
        B::Erro: ParaMotivo + Display,
        F: FnOnce() -> Result<Preparo<B>, Motivo> + Send + 'static,
    {
        self.iniciar_medido(
            identidade,
            emitir,
            Arc::new(crate::tempo_de_impressao::RelogioDoSistema::default()),
            Box::new(|_| {}),
            preparar,
        )
    }

    /// Como `iniciar`, dizendo como medir o tempo e a quem contar como o trabalho acabou.
    pub fn iniciar_medido<B, F>(
        &self,
        identidade: Identidade,
        emitir: Emissor,
        relogio: Arc<dyn Relogio>,
        fim: FimDoTrabalho,
        preparar: F,
    ) -> bool
    where
        B: Backend + DescreveFalha,
        B::Erro: ParaMotivo + Display,
        F: FnOnce() -> Result<Preparo<B>, Motivo> + Send + 'static,
    {
        let mut trabalhos = self
            .trabalhos
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        // Trabalho em erro so espera Reimprimir ou Cancelar: quem pede um novo o substitui, e o
        // antigo acaba quando o recado dele deixa de ter quem o mande.
        let ocupada = trabalhos
            .get(&identidade.impressora)
            .is_some_and(|entrada| {
                entrada.ativo.load(Ordering::SeqCst) && !entrada.substituivel.load(Ordering::SeqCst)
            });
        if ocupada {
            return false;
        }
        let (envia, recebe) = channel();
        let ativo = Arc::new(AtomicBool::new(true));
        let substituivel = Arc::new(AtomicBool::new(false));
        let substituivel_da_thread = Arc::clone(&substituivel);
        let guarda = GuardaDaThread {
            identidade: identidade.clone(),
            emitir: Arc::clone(&emitir),
            ativo: Arc::clone(&ativo),
            fim: Some(fim),
            desfecho: None,
        };
        emitir(identidade.cartao(EstadoCartao::Preparando));
        let chave = identidade.impressora.clone();
        let aba = identidade.aba;
        let (identidade_fora, emitir_fora) = (identidade.clone(), Arc::clone(&emitir));
        let iniciada = std::thread::Builder::new()
            .name("trabalho-de-impressao".into())
            .spawn(move || {
                let mut guarda = guarda;
                guarda.desfecho = Some(match preparar() {
                    Ok(preparo) => conduzir(
                        preparo,
                        &identidade,
                        &recebe,
                        &emitir,
                        &*relogio,
                        substituivel_da_thread,
                    ),
                    Err(motivo) => {
                        // Nada foi enviado: o cartao em erro nao segura a impressora.
                        substituivel_da_thread.store(true, Ordering::SeqCst);
                        recusado(&identidade, &recebe, &emitir, motivo);
                        Desfecho::sem_envio()
                    }
                });
            });
        if iniciada.is_err() {
            ativo.store(false, Ordering::SeqCst);
            emitir_fora(identidade_fora.cartao(EstadoCartao::Erro {
                motivo: motivo_de_interrupcao(),
            }));
            return true;
        }
        trabalhos.insert(
            chave,
            Entrada {
                aba,
                recados: envia,
                ativo,
                substituivel,
            },
        );
        true
    }

    /// Entrega o comando. `Err` quer dizer que nenhuma thread vai atender.
    pub fn comandar(&self, impressora: &str, comando: Comando) -> Result<(), ()> {
        let trabalhos = self
            .trabalhos
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let entrada = trabalhos.get(impressora).ok_or(())?;
        entrada
            .recados
            .send(Recado::Comando(comando))
            .map_err(|_| ())
    }

    /// Os trabalhos deste documento param de ser acompanhados. Nada e apagado da fila.
    pub fn soltar_aba(&self, aba: u64) {
        let trabalhos = self
            .trabalhos
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for entrada in trabalhos.values().filter(|entrada| entrada.aba == aba) {
            let _ = entrada.recados.send(Recado::Soltar);
        }
    }

    /// O programa vai fechar: ninguem mais e acompanhado. Espera as threads contarem como foi, ate o
    /// prazo: uma passada sendo desenhada pode levar minutos, e a janela nao fica presa nela.
    pub fn soltar_todos_e_esperar(&self, prazo: Duration) {
        let inicio = std::time::Instant::now();
        let ativos: Vec<Arc<AtomicBool>> = self
            .trabalhos
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .map(|entrada| {
                let _ = entrada.recados.send(Recado::Soltar);
                Arc::clone(&entrada.ativo)
            })
            .collect();
        while ativos.iter().any(|ativo| ativo.load(Ordering::SeqCst)) && inicio.elapsed() < prazo {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

/// A preparacao falhou: nada foi enviado. O cartao mostra o motivo ate o usuario pedir de novo.
fn recusado(identidade: &Identidade, recados: &Receiver<Recado>, emitir: &Emissor, motivo: Motivo) {
    emitir(identidade.cartao(EstadoCartao::Erro { motivo }));
    loop {
        match recados.recv() {
            Ok(Recado::Comando(Comando::Cancelar)) => {
                emitir(identidade.cartao(EstadoCartao::Cancelado));
                return;
            }
            Ok(Recado::Comando(_)) => {}
            Ok(Recado::Soltar) | Err(_) => return,
        }
    }
}

/// Como a interface pede que os lotes sejam distribuidos entre as impressoras.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModoDoEnvio {
    Espelho,
    Divisao,
}

impl From<ModoDoEnvio> for ModoDistribuicao {
    fn from(modo: ModoDoEnvio) -> Self {
        match modo {
            ModoDoEnvio::Espelho => Self::Espelho,
            ModoDoEnvio::Divisao => Self::Divisao,
        }
    }
}

impl From<ModoDistribuicao> for ModoDoEnvio {
    fn from(modo: ModoDistribuicao) -> Self {
        match modo {
            ModoDistribuicao::Espelho => Self::Espelho,
            ModoDistribuicao::Divisao => Self::Divisao,
        }
    }
}

/// O caminho do PDF e o que a pessoa escolheu. O trecho so e conferido ao ler o PDF, quando o total
/// de paginas e conhecido; sem trecho, vale o documento inteiro.
#[derive(Debug, Clone)]
pub struct Pedido {
    pub caminho: String,
    pub intervalo: Option<IntervaloPaginas>,
    pub alcance: Alcance,
    pub papel_de_ajuste: Option<plots_core::ajuste::PapelDeAjuste>,
    pub controle: Option<IdiomaDaMarca>,
    pub driver_da_simulada: DriverDaSimulada,
    /// Cópias do documento: cada uma sai inteira, com as suas folhas fechadas.
    pub copias: usize,
    /// Impressão do documento guardada pela frente desta aba: o verso só sai se o arquivo ainda for o mesmo.
    pub impressao_esperada: Option<String>,
    /// Aba do documento: a frente que prepara com este pedido guarda a impressão que leu para ela.
    pub aba: Option<u64>,
}

/// Modo de teste da simulada: imita um driver que inverte, com ou sem a opcao do PlotS ligada.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DriverDaSimulada {
    pub inverte: bool,
    pub compensa: bool,
}

impl Pedido {
    pub fn novo(caminho: String, intervalo: Option<IntervaloPaginas>) -> Self {
        Self {
            caminho,
            intervalo,
            alcance: Alcance::default(),
            papel_de_ajuste: None,
            controle: None,
            driver_da_simulada: DriverDaSimulada::default(),
            copias: 1,
            impressao_esperada: None,
            aba: None,
        }
    }

    pub fn com_impressao_esperada(mut self, impressao: Option<String>) -> Self {
        self.impressao_esperada = impressao;
        self
    }

    pub fn com_aba(mut self, aba: u64) -> Self {
        self.aba = Some(aba);
        self
    }

    pub fn com_copias(mut self, copias: usize) -> Self {
        self.copias = copias;
        self
    }

    pub fn com_alcance(mut self, alcance: Alcance) -> Self {
        self.alcance = alcance;
        self
    }

    pub fn com_papel_de_ajuste(mut self, papel: Option<plots_core::ajuste::PapelDeAjuste>) -> Self {
        self.papel_de_ajuste = papel;
        self
    }

    pub fn com_controle(mut self, controle: Option<IdiomaDaMarca>) -> Self {
        self.controle = controle;
        self
    }

    pub fn com_driver_da_simulada(mut self, driver: DriverDaSimulada) -> Self {
        self.driver_da_simulada = driver;
        self
    }
}

fn motivo_do_pdf(erro: &documento::ErroDocumento) -> Motivo {
    Motivo::novo("pdf", format!("não foi possível preparar {}", erro.nome))
}

#[cfg(test)]
pub(crate) fn planejar(
    dimensoes: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
    folhas_por_lote: usize,
    perfil: PerfilImpressora,
) -> Result<Vec<LoteDoFluxo>, Motivo> {
    let lotes = plots_core::plano_lotes_do_intervalo(
        dimensoes,
        intervalo,
        folhas_por_lote,
        OrdemPassadas::default(),
        perfil,
    )
    .map_err(|erro| Motivo::novo("pdf", erro.to_string()))?;
    Ok(fluxo::lotes_do_plano(&lotes))
}

/// A face em branco de uma folha toma o tamanho da ultima pagina que o envio conhece. Parar a lista
/// no fim do trecho faz essa pagina ser a frente da propria folha, e nao o fim do documento, que
/// pode ser de outro papel. As paginas seguintes nunca sao enviadas.
#[cfg_attr(not(windows), allow(dead_code))]
fn dimensoes_do_envio(
    mut dimensoes: Vec<DimensoesPagina>,
    intervalo: Option<IntervaloPaginas>,
) -> Vec<DimensoesPagina> {
    if let Some(intervalo) = intervalo {
        dimensoes.truncate(intervalo.fim);
    }
    dimensoes
}

/// O plano de cada impressora sai do perfil dela: um lote por trecho do mesmo papel, sem parar no
/// meio. Com `automatico`, os papeis aceitos saem nas duas faces, em um trabalho por trecho.
pub(crate) fn planejar_pedido(
    dimensoes: &[DimensoesPagina],
    pedido: &Pedido,
    perfil: PerfilImpressora,
    automatico: bool,
) -> Result<Vec<LoteDoFluxo>, Motivo> {
    let ajustadas = pedido
        .papel_de_ajuste
        .map(|papel| papel.dimensoes_no_papel(dimensoes));
    let medidas = ajustadas.as_deref().unwrap_or(dimensoes);
    let ordem = OrdemPassadas::default();
    let plano = if automatico {
        plano_automatico_copias_do_intervalo(
            medidas,
            pedido.intervalo,
            pedido.copias,
            ordem,
            perfil,
            RepartoAutomatico::PorTrechoDePapel,
        )
    } else {
        plano_copias_do_intervalo(medidas, pedido.intervalo, pedido.copias, ordem, perfil)
    }
    .map_err(|erro| Motivo::novo("pdf", erro.to_string()))?;
    let mut lotes = fluxo::lotes_do_plano(&plano);
    if let Some(idioma) = pedido.controle {
        marcar_lotes(&mut lotes, idioma);
    }
    Ok(lotes)
}

/// Impressão do PDF no disco: o conteúdo inteiro, por FNV-1a de 64 bits, com o tamanho. Qualquer
/// troca do arquivo muda este texto, inclusive a medida de uma página. Produção guarda a impressão
/// que a frente leu; aqui só serve para conferir o que o disco tinha.
#[cfg(test)]
pub fn impressao_do_arquivo(caminho: &str) -> Result<String, Motivo> {
    use std::io::Read;
    let falha = || Motivo::novo("pdf", "não foi possível reler o documento");
    let mut arquivo = std::fs::File::open(caminho).map_err(|_| falha())?;
    let mut bloco = [0u8; 65536];
    let mut impressao = leitura_pdf::Impressao::default();
    loop {
        let lidos = arquivo.read(&mut bloco).map_err(|_| falha())?;
        if lidos == 0 {
            break;
        }
        impressao.acrescentar(bloco.get(..lidos).unwrap_or(&[]));
    }
    Ok(impressao.texto())
}

/// Impressões das frentes de cada aba: o verso dela só sai se o arquivo ainda for o mesmo.
fn impressoes_das_frentes(
) -> std::sync::MutexGuard<'static, std::collections::BTreeMap<u64, String>> {
    static IMPRESSOES: Mutex<std::collections::BTreeMap<u64, String>> =
        Mutex::new(std::collections::BTreeMap::new());
    IMPRESSOES.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A impressão que a frente guardou para a aba. É a que o verso da aba precisa conferir; aba que não
/// teve frente aceita não tem nenhuma.
pub fn impressao_guardada(aba: u64) -> Option<String> {
    impressoes_das_frentes().get(&aba).cloned()
}

/// Guarda a impressão que a frente da aba leu. Só é chamada depois que o preparo da frente foi aceito:
/// uma frente recusada não troca a impressão que o verso confere. Com várias impressoras, a última
/// frente aceita prevalece.
pub fn guardar_impressao_da_frente(aba: u64, impressao: String) {
    impressoes_das_frentes().insert(aba, impressao);
}

/// Guarda o que a frente leu do PDF, quando o pedido é de frentes e tem aba. Versos não mudam a guarda.
fn guardar_frente_lida(pedido: &Pedido, impressao: &str) {
    if let (true, Some(aba)) = (matches!(pedido.alcance, Alcance::SoFrentes), pedido.aba) {
        guardar_impressao_da_frente(aba, impressao.to_owned());
    }
}

/// Esquece a impressão da frente de uma aba que foi fechada: o verso dela não confere mais nada.
pub fn esquecer_impressao(aba: u64) {
    impressoes_das_frentes().remove(&aba);
}

/// Antes de planejar o PDF, o verso compara a impressão dos bytes que ele carrega com a da frente: se
/// mudou, nada é planejado nem enviado.
fn verificar_impressao(pedido: &Pedido, impressao_lida: &str) -> Result<(), Motivo> {
    let Some(esperada) = pedido.impressao_esperada.as_deref() else {
        return Ok(());
    };
    if impressao_lida == esperada {
        return Ok(());
    }
    Err(Motivo::novo("documento_mudou", ""))
}

pub fn preparar_simulada(
    modelo: ModeloImpressora,
    pedido: &Pedido,
) -> Result<Preparo<DriverDeTeste<SimGuiado>>, Motivo> {
    let lido = leitura_pdf::ler_pdf(&pedido.caminho).map_err(|erro| motivo_do_pdf(&erro))?;
    verificar_impressao(pedido, &lido.impressao)?;
    let mut lotes = planejar_pedido(&lido.dimensoes, pedido, modelo.perfil(), false)?;
    let driver = pedido.driver_da_simulada;
    if driver.compensa {
        compensar_driver(&mut lotes);
    }
    let backend = DriverDeTeste::novo(
        SimGuiado::novo(Mecanica::do_modelo(modelo), modelo.gesto().into()),
        driver.inverte,
    );
    let preparo = preparo_simulado(modelo, backend, lotes, pedido.alcance)?;
    guardar_frente_lida(pedido, &lido.impressao);
    Ok(preparo)
}

/// A simulada que imita uma impressora de duas faces: confere as folhas, recusa o papel que a
/// referencia recusa e usa o gesto de `modelo` nos outros papeis.
pub fn preparar_simulada_automatica(
    modelo: ModeloImpressora,
    pedido: &Pedido,
) -> Result<Preparo<DriverDeTeste<SimGuiado>>, Motivo> {
    let lido = leitura_pdf::ler_pdf(&pedido.caminho).map_err(|erro| motivo_do_pdf(&erro))?;
    verificar_impressao(pedido, &lido.impressao)?;
    let lotes = planejar_pedido(&lido.dimensoes, pedido, modelo.perfil(), true)?;
    // A referencia julga o papel que de fato vai a impressora, nao o tamanho original da pagina.
    let medidas = match pedido.papel_de_ajuste {
        Some(papel) => papel.dimensoes_no_papel(&lido.dimensoes),
        None => lido.dimensoes,
    };
    let backend = DriverDeTeste::novo(
        SimGuiado::automatica(Mecanica::do_modelo(modelo), modelo.gesto().into(), medidas),
        false,
    );
    let preparo = preparo_simulado(modelo, backend, lotes, pedido.alcance)?;
    guardar_frente_lida(pedido, &lido.impressao);
    Ok(preparo)
}

fn preparo_simulado(
    modelo: ModeloImpressora,
    backend: DriverDeTeste<SimGuiado>,
    lotes: Vec<LoteDoFluxo>,
    alcance: Alcance,
) -> Result<Preparo<DriverDeTeste<SimGuiado>>, Motivo> {
    let condutor = Condutor::novo(backend, lotes, ESPERA_SIMULADA)
        .map_err(|erro| motivo_do_job(&erro))?
        .com_alcance(alcance);
    let vira_para_ler = fluxo::vira_para_ler(modelo) && condutor.tem_lote_manual();
    Ok(Preparo {
        condutor,
        recolocar: Recolocar::da_instrucao(Instrucao::do_modelo(modelo)),
        vira_para_ler,
    })
}

/// Fila real: confere de novo tudo o que a interface ja filtrou, porque a lista dela pode estar
/// velha. Nenhuma pagina sai daqui; o envio so comeca depois.
#[cfg(windows)]
pub fn preparar_fila(
    nome: &str,
    pedido: &Pedido,
) -> Result<Preparo<plots_windows::BackendWindows>, Motivo> {
    let perfis = impressoras::ler_perfis_do_sistema()
        .map_err(|erro| Motivo::novo("perfil", erro.to_string()))?;
    let entrada = perfis.obter(nome).ok_or_else(|| {
        Motivo::novo(
            "perfil",
            format!("a impressora '{nome}' não tem perfil salvo"),
        )
    })?;
    let lido = leitura_pdf::ler_pdf(&pedido.caminho).map_err(|erro| motivo_do_pdf(&erro))?;
    verificar_impressao(pedido, &lido.impressao)?;
    let mut lotes = planejar_pedido(&lido.dimensoes, pedido, entrada.perfil, entrada.automatico)?;
    // So os lotes da rota manual mudam: quem sai em duas faces nao depende da ordem do driver.
    if entrada.driver_inverte {
        compensar_driver(&mut lotes);
    }
    let pasta = impressoras::pasta_de_saida_do_sistema();
    let filas =
        plots_windows::listar_filas().map_err(|erro| Motivo::novo("sistema", erro.to_string()))?;
    let fila = plots_windows::decidir_fila(&filas, nome, pasta.is_some())
        .map_err(|recusa| motivo_da_recusa(nome, recusa))?;
    if impressoras::fila_com_problema(fila) {
        return Err(Motivo::novo(
            "fila",
            format!("a fila '{nome}' não pode receber agora; veja o estado dela na lista"),
        ));
    }
    let dimensoes = dimensoes_do_envio(lido.dimensoes, pedido.intervalo);
    let dimensoes = pedido.papel_de_ajuste.map_or(dimensoes.clone(), |papel| {
        papel.dimensoes_no_papel(&dimensoes)
    });
    let mut backend =
        plots_windows::BackendWindows::novo(fila, lido.origem, dimensoes, pasta.as_deref())
            .map_err(|erro| erro.para_motivo())?;
    backend.definir_papel_de_saida(pedido.papel_de_ajuste.map(|papel| papel.medida_mm()));
    backend.definir_marcas(lotes.iter().map(|item| item.marcas.clone()).collect());
    let condutor = Condutor::novo(backend, lotes, fluxo::ESPERA_REAL)
        .map_err(|erro| motivo_do_job(&erro))?
        .com_alcance(pedido.alcance);
    let (recolocar, vira_para_ler) = gesto_e_leitura(entrada, perfis.calibracao(nome));
    let vira_para_ler = vira_para_ler && condutor.tem_lote_manual();
    guardar_frente_lida(pedido, &lido.impressao);
    Ok(Preparo {
        condutor,
        recolocar,
        vira_para_ler,
    })
}

/// O gesto e se o maco e lido virado. O que a calibracao mediu vale sobre o que o modelo diria.
pub(crate) fn gesto_e_leitura(
    entrada: &plots_perfis::EntradaPerfil,
    calibracao: Option<plots_perfis::Calibracao>,
) -> (Recolocar, bool) {
    match calibracao {
        Some(calibrada) => (
            Recolocar::calibrado(entrada.perfil, calibrada.face_impressa_entra_para_cima),
            calibrada.le_virando,
        ),
        None => (
            recolocar_do_perfil(entrada.modelo, entrada.gesto_texto.clone()),
            entrada.modelo.is_some_and(fluxo::vira_para_ler),
        ),
    }
}

/// Recusa de fila em um sistema que nao tem spooler.
#[cfg_attr(windows, allow(dead_code))]
pub fn fila_indisponivel() -> Motivo {
    Motivo::novo(
        "sistema",
        "o envio à impressora só está disponível no Windows",
    )
}

/// Com modelo, o gesto e o dele; sem modelo, vale o que foi feito na calibracao.
pub(crate) fn recolocar_do_perfil(
    modelo: Option<ModeloImpressora>,
    texto: Option<String>,
) -> Recolocar {
    match modelo {
        Some(modelo) => Recolocar::da_instrucao(Instrucao::do_modelo(modelo)),
        None => Recolocar {
            lado: None,
            virada: None,
            texto_do_perfil: texto,
        },
    }
}

/// Sem arquivo de saida, uma fila que pergunta o nome do arquivo e recusada: o sistema abriria um
/// dialogo e o trabalho ficaria parado.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn motivo_da_recusa(nome: &str, recusa: plots_windows::RecusaDeFila) -> Motivo {
    use plots_windows::RecusaDeFila as R;
    match recusa {
        R::Inexistente => Motivo::novo(
            "fila_inexistente",
            format!("a fila '{nome}' não existe neste sistema"),
        ),
        R::Fax => Motivo::novo(
            "fila_fax",
            format!("a fila '{nome}' é um fax compartilhado; o envio não usa fax"),
        ),
        R::PedeArquivo => Motivo::novo(
            "fila_pede_arquivo",
            format!("a fila '{nome}' pede o nome do arquivo em um diálogo, e esta versão não abre esse diálogo"),
        ),
    }
}

/// O que a confirmacao mostra antes do envio.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumoDoEnvio {
    pub paginas: usize,
    pub lotes: usize,
    pub papeis: Vec<PapelDoCartao>,
    pub pares_mistos: Vec<(usize, usize)>,
    /// Pares de páginas de tamanhos diferentes: cada uma sai na sua folha, com o verso em branco.
    pub pares_separados: Vec<(usize, usize)>,
    pub assinado: bool,
    /// O gesto de cada impressora, para a confirmacao do verso mostrar antes de enviar.
    pub gestos: Vec<GestoDaImpressora>,
    /// De qual a qual pagina vai cada lote, com os numeros do documento.
    pub faixas: Vec<FaixaDoLote>,
    /// Em uma impressora de duas faces, quais paginas saem sozinhas e quais pedem o gesto.
    pub fluxos: Vec<TrechoDoFluxo>,
    /// Quantos trabalhos essa impressora faz fora da divisao.
    pub lotes_automaticos: usize,
    /// Folhas do envio inteiro: no espelho, de cada impressora.
    pub folhas: usize,
    /// O maior maco que alguem vai ter de recolocar de uma vez.
    pub maior_maco: usize,
    /// Intervalo das escalas aplicadas às páginas, quando há papel de destino.
    pub escalas: Option<(f64, f64)>,
    /// Qual impressora termina antes, quando o historico tem dados de todas e a diferenca e grande.
    pub comparacao: Option<ComparacaoDoResumo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GestoDaImpressora {
    pub impressora: String,
    pub recolocar: Recolocar,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComparacaoDoResumo {
    pub rapida: String,
    pub lenta: String,
    pub razao: f64,
}

/// Paginas seguidas que seguem pelo mesmo fluxo, em ordem de documento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TrechoDoFluxo {
    pub automatico: bool,
    pub inicio: usize,
    pub fim: usize,
}

/// Junta os lotes vizinhos do mesmo fluxo. Lote sem pagina numerada nao entra.
pub fn trechos_dos_fluxos(lotes: &[LotePapel]) -> Vec<TrechoDoFluxo> {
    let mut trechos: Vec<TrechoDoFluxo> = Vec::new();
    for lote in lotes {
        let numeros = lote.paginas.iter().filter_map(|pagina| *pagina);
        let (Some(inicio), Some(fim)) = (numeros.clone().min(), numeros.max()) else {
            continue;
        };
        match trechos.last_mut() {
            Some(anterior) if anterior.automatico == lote.duas_faces => anterior.fim = fim,
            _ => trechos.push(TrechoDoFluxo {
                automatico: lote.duas_faces,
                inicio,
                fim,
            }),
        }
    }
    trechos
}

fn folhas_do_lote(lote: &LotePapel) -> usize {
    lote.paginas.len().div_ceil(2)
}

/// As paginas de um lote; o verso em branco do fim nao conta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FaixaDoLote {
    pub lote: usize,
    pub inicio: usize,
    pub fim: usize,
}

/// Os lotes sao numerados a partir de 1, na ordem do plano. Lote sem pagina numerada nao tem faixa.
pub fn faixas_dos_lotes(lotes: &[LotePapel]) -> Vec<FaixaDoLote> {
    lotes
        .iter()
        .enumerate()
        .filter_map(|(indice, lote)| {
            let numeros = lote.paginas.iter().filter_map(|pagina| *pagina);
            Some(FaixaDoLote {
                lote: indice.saturating_add(1),
                inicio: numeros.clone().min()?,
                fim: numeros.max()?,
            })
        })
        .collect()
}

/// Lotes e papeis nao dependem do perfil: o plano padrao diz quantos sao. Com um trecho, tudo aqui
/// conta so o trecho.
#[cfg(test)]
pub fn resumo_do_envio(
    caminho: &str,
    intervalo: Option<IntervaloPaginas>,
) -> Result<ResumoDoEnvio, documento::ErroDocumento> {
    resumo_do_envio_com_papel(caminho, intervalo, None)
}

// So os testes chamam esta funcao hoje; o comando da janela passa por `resumo_do_envio_com_copias`.
#[cfg_attr(not(test), allow(dead_code))]
pub fn resumo_do_envio_com_papel(
    caminho: &str,
    intervalo: Option<IntervaloPaginas>,
    papel: Option<plots_core::ajuste::PapelDeAjuste>,
) -> Result<ResumoDoEnvio, documento::ErroDocumento> {
    resumo_do_envio_com_copias(caminho, intervalo, papel, 1)
}

/// O resumo de um envio com `copias` cópias do trecho: folhas, maço e faixas contam todas elas.
pub fn resumo_do_envio_com_copias(
    caminho: &str,
    intervalo: Option<IntervaloPaginas>,
    papel: Option<plots_core::ajuste::PapelDeAjuste>,
    copias: usize,
) -> Result<ResumoDoEnvio, documento::ErroDocumento> {
    let lido = leitura_pdf::ler_pdf(caminho)?;
    let falha = || documento::falha_inesperada(caminho);
    let escalas = papel
        .map(|formato| formato.faixa_de_escala(&lido.dimensoes, intervalo))
        .transpose()
        .map_err(|_| falha())?
        .flatten();
    let dimensoes_ajustadas = papel.map(|formato| formato.dimensoes_no_papel(&lido.dimensoes));
    let medidas = dimensoes_ajustadas.as_deref().unwrap_or(&lido.dimensoes);
    let lotes = plano_copias_do_intervalo(
        medidas,
        intervalo,
        copias,
        OrdemPassadas::default(),
        PerfilImpressora::default(),
    )
    .map_err(|_| falha())?;
    let automatico = plano_automatico_copias_do_intervalo(
        medidas,
        intervalo,
        copias,
        OrdemPassadas::default(),
        PerfilImpressora::default(),
        RepartoAutomatico::PorTrechoDePapel,
    )
    .map_err(|_| falha())?;
    let pares_mistos =
        plots_core::pares_mistos_do_intervalo(&lido.dimensoes, intervalo).map_err(|_| falha())?;
    let pares_separados =
        plots_core::pares_separados_do_intervalo(medidas, intervalo).map_err(|_| falha())?;
    let mut papeis: Vec<&Papel> = Vec::new();
    for lote in &lotes {
        if !papeis
            .iter()
            .any(|papel| fluxo::mesma_folha(papel, &lote.papel))
        {
            papeis.push(&lote.papel);
        }
    }
    Ok(ResumoDoEnvio {
        paginas: intervalo.map_or(lido.dimensoes.len(), IntervaloPaginas::quantidade),
        lotes: lotes.len(),
        papeis: papeis.into_iter().map(papel_do_cartao).collect(),
        pares_mistos,
        pares_separados,
        assinado: lido.assinado,
        gestos: Vec::new(),
        faixas: faixas_dos_lotes(&lotes),
        fluxos: trechos_dos_fluxos(&automatico),
        lotes_automaticos: automatico.len(),
        folhas: lotes.iter().map(folhas_do_lote).sum(),
        maior_maco: lotes.iter().map(folhas_do_lote).max().unwrap_or(0),
        escalas,
        comparacao: None,
    })
}

#[cfg(test)]
#[path = "trabalhos_testes.rs"]
mod testes;
