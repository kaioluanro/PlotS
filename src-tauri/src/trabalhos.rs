//! Um trabalho por impressora, cada um em uma thread propria, fora da thread da interface.
//!
//! Cada thread conduz um documento em uma impressora e publica o cartao a cada mudanca. A interface
//! manda comandos por um canal; impressoras diferentes nunca esperam umas pelas outras. Nada aqui
//! tira um trabalho da fila sem um pedido do usuario.

use crate::documento;
use crate::impressoras;
use crate::leitura_pdf;
use plots_core::distribuicao::{distribuir_lotes, ModoDistribuicao};
use plots_core::{
    plano_lotes_do_intervalo, DimensoesPagina, FormatoPapel, IntervaloPaginas, LotePapel,
    ModeloImpressora, OrdemPassadas, Pagina, Papel, PerfilImpressora,
};
use plots_windows::plots_print::fluxo::{
    self, CondicaoDoAviso, Condutor, DescreveFalha, Espera, Instrucao, Lado, LoteDoFluxo, Passo,
    SimGuiado, Virada,
};
use plots_windows::plots_print::job::{Backend, ErroJob, EstadoJob};
use plots_windows::plots_print::{ErroSimulacao, Mecanica};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// Maior lote que a interface oferece; um valor de fora nunca chega ao plano.
const LOTE_MAXIMO: usize = 50;
/// A simulacao nao tem fila: o intervalo so deixa o estado aparecer antes da confirmacao.
const ESPERA_SIMULADA: Espera = Espera {
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
    fn novo(codigo: &'static str, texto: impl Into<String>) -> Self {
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

impl ParaMotivo for ErroSimulacao {
    fn para_motivo(&self) -> Motivo {
        Motivo::novo("sistema", self.to_string())
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
            E::PastaDeSaidaInvalida(_) | E::ArquivoJaExiste(_) | E::ArquivoNaoGerado(_) => {
                "arquivo_saida"
            }
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

fn motivo_do_job<E: ParaMotivo>(erro: &ErroJob<E>) -> Motivo {
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
        Self {
            lado: Some(match instrucao.lado {
                Lado::Cima => "cima",
                Lado::Baixo => "baixo",
            }),
            virada: Some(match instrucao.virada {
                Virada::PelaBordaDeCima => "borda_de_cima",
                Virada::PelaLateral => "lateral",
                Virada::NaoVirar => "nao_virar",
                Virada::Girar => "girar",
            }),
            texto_do_perfil: None,
        }
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
}

impl<B> Andamento<'_, B>
where
    B: Backend + DescreveFalha,
    B::Erro: ParaMotivo + Display,
{
    fn publicar(&self, estado: EstadoCartao) {
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
        cartao.troca_de_papel = self
            .condutor
            .troca_de_papel()
            .map(|papel| papel_do_cartao(&papel));
        if self.identidade.simulada {
            cartao.sequencia = self.condutor.lote().map(|item| Sequencia {
                frente: faces(&item.lote.passada1),
                verso: faces(&item.lote.passada2),
            });
        }
        (self.emitir)(cartao);
    }

    fn publicar_erro(&self, motivo: Motivo) {
        self.publicar(EstadoCartao::Erro { motivo });
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

    fn enviar(&mut self, passada: PassadaDoCartao) {
        self.aviso = None;
        self.parado = false;
        self.passada = passada;
        // O estado aparece antes: desenhar as paginas de uma passada pode levar minutos.
        self.publicar(EstadoCartao::Imprimindo { passada });
        let enviado = match passada {
            PassadaDoCartao::Frente => self.condutor.enviar_frente(),
            PassadaDoCartao::Verso => self.condutor.enviar_verso(),
        };
        let recusou =
            !self.identidade.simulada && crate::preferencias::tomar_recusa(&self.identidade.nome);
        self.ajuste_recusado |= recusou;
        match enviado {
            Err(erro) => self.publicar_erro(motivo_do_job(&erro)),
            // A recusa aparece logo: o proximo estado pode demorar a mudar.
            Ok(_) if recusou => self.publicar(EstadoCartao::Imprimindo { passada }),
            Ok(_) => {}
        }
    }

    fn somar_resultado(&mut self) {
        let Some(item) = self.condutor.lote_terminado() else {
            return;
        };
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
            EstadoJob::FrenteEnviada | EstadoJob::VersoEnviado | EstadoJob::Erro
        );
        if tem_passada_na_fila {
            if let Err(erro) = self.condutor.cancelar() {
                self.falhou_ao_cancelar(&erro);
                return Continua::Sim;
            }
        }
        self.publicar(EstadoCartao::Cancelado);
        Continua::Nao
    }

    fn atender(&mut self, comando: Comando) -> Continua {
        let estado = self.condutor.estado();
        match comando {
            Comando::EnviarFrente if estado == EstadoJob::Pronto => {
                self.enviar(PassadaDoCartao::Frente);
            }
            Comando::EnviarVerso if estado == EstadoJob::AguardandoRecolocacao => {
                self.enviar(PassadaDoCartao::Verso);
            }
            // Com aviso, o trabalho ainda esta na fila: refazer o lote comeca por tira-lo de la.
            Comando::Reimprimir
                if estado == EstadoJob::Erro || self.parado || self.aviso.is_some() =>
            {
                self.aviso = None;
                self.parado = false;
                self.passada = PassadaDoCartao::Frente;
                self.publicar(EstadoCartao::Imprimindo {
                    passada: PassadaDoCartao::Frente,
                });
                // Antes de mandar a frente de novo, o trabalho anterior sai da fila; se a fila recusar, nada e enviado.
                if let Err(erro) = self.condutor.cancelar() {
                    self.falhou_ao_cancelar(&erro);
                } else if let Err(erro) = self.condutor.enviar_frente() {
                    self.parado = true;
                    self.publicar_erro(motivo_do_job(&erro));
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
        match self.condutor.consultar() {
            Passo::EmAndamento => {
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
                self.publicar(EstadoCartao::Aguardando);
            }
            Passo::LoteConcluido => {
                self.aviso = None;
                self.somar_resultado();
                self.publicar(EstadoCartao::ProximoLote);
            }
            Passo::DocumentoConcluido => {
                self.somar_resultado();
                self.publicar(EstadoCartao::Concluido);
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
                EstadoJob::FrenteEnviada | EstadoJob::VersoEnviado
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
) where
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
    };
    // Os recados chegados durante a preparacao valem antes do primeiro envio: soltar a aba ou
    // cancelar nesse intervalo nao pode mandar a frente para a fila.
    loop {
        match recados.try_recv() {
            Ok(Recado::Soltar) | Err(TryRecvError::Disconnected) => return,
            Ok(Recado::Comando(Comando::Cancelar)) => {
                andamento.publicar(EstadoCartao::Cancelado);
                return;
            }
            Ok(Recado::Comando(_)) => {}
            Err(TryRecvError::Empty) => break,
        }
    }
    andamento.enviar(PassadaDoCartao::Frente);
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
}

impl Drop for GuardaDaThread {
    fn drop(&mut self) {
        self.ativo.store(false, Ordering::SeqCst);
        if std::thread::panicking() {
            (self.emitir)(self.identidade.cartao(EstadoCartao::Erro {
                motivo: motivo_de_interrupcao(),
            }));
        }
    }
}

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
}

/// Os trabalhos em andamento, um por impressora.
#[derive(Default)]
pub struct Gerente {
    trabalhos: Mutex<HashMap<String, Entrada>>,
}

impl Gerente {
    /// Inicia o trabalho em uma thread propria. Devolve `false`, sem mudar nada, se a impressora
    /// ja tem um trabalho em andamento.
    pub fn iniciar<B, F>(&self, identidade: Identidade, emitir: Emissor, preparar: F) -> bool
    where
        B: Backend + DescreveFalha,
        B::Erro: ParaMotivo + Display,
        F: FnOnce() -> Result<Preparo<B>, Motivo> + Send + 'static,
    {
        let mut trabalhos = self
            .trabalhos
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let ocupada = trabalhos
            .get(&identidade.impressora)
            .is_some_and(|entrada| entrada.ativo.load(Ordering::SeqCst));
        if ocupada {
            return false;
        }
        let (envia, recebe) = channel();
        let ativo = Arc::new(AtomicBool::new(true));
        let guarda = GuardaDaThread {
            identidade: identidade.clone(),
            emitir: Arc::clone(&emitir),
            ativo: Arc::clone(&ativo),
        };
        emitir(identidade.cartao(EstadoCartao::Preparando));
        let chave = identidade.impressora.clone();
        let aba = identidade.aba;
        let (identidade_fora, emitir_fora) = (identidade.clone(), Arc::clone(&emitir));
        let iniciada = std::thread::Builder::new()
            .name("trabalho-de-impressao".into())
            .spawn(move || {
                let _guarda = guarda;
                match preparar() {
                    Ok(preparo) => conduzir(preparo, &identidade, &recebe, &emitir),
                    Err(motivo) => recusado(&identidade, &recebe, &emitir, motivo),
                }
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
}

/// A preparacao falhou: nada foi enviado. O cartao mostra o motivo ate o usuario pedir de novo.
fn recusado(identidade: &Identidade, recados: &Receiver<Recado>, emitir: &Emissor, motivo: Motivo) {
    // Nada foi enviado e nada resta a acompanhar: a thread acaba, e a impressora fica livre.
    let sem_trabalho = motivo.codigo == MOTIVO_DOCUMENTO_MUDOU;
    emitir(identidade.cartao(EstadoCartao::Erro { motivo }));
    if sem_trabalho {
        return;
    }
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

/// O documento nao e mais o que a confirmacao mostrou; nenhuma pagina saiu.
pub const MOTIVO_DOCUMENTO_MUDOU: &str = "documento_mudou";

fn documento_mudou() -> Motivo {
    Motivo::novo(
        MOTIVO_DOCUMENTO_MUDOU,
        "o documento mudou depois da confirmação. Nada foi enviado: envie de novo.",
    )
}

/// Na divisao, a posicao desta impressora no rodizio, e os lotes e as paginas que foram confirmados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParteDoEnvio {
    pub indice: usize,
    pub impressoras: usize,
    pub lotes: usize,
    /// Paginas do documento inteiro, nao so do trecho.
    pub paginas: usize,
}

impl ParteDoEnvio {
    /// Os lotes desta impressora, ja com a ordem do perfil dela. Se o documento mudou depois da
    /// confirmacao, nada sai: o cartao diria um lote e o maco traria outro.
    pub fn separar(&self, lotes: Vec<LoteDoFluxo>) -> Result<Vec<LoteDoFluxo>, Motivo> {
        if lotes.len() != self.lotes {
            return Err(documento_mudou());
        }
        let plano = distribuir_lotes(lotes.len(), self.impressoras, ModoDistribuicao::Divisao)
            .map_err(|erro| Motivo::novo("sistema", erro.to_string()))?;
        let atribuicao = plano.get(self.indice).ok_or_else(|| {
            Motivo::novo("sistema", "a impressora ficou fora da divisão dos lotes")
        })?;
        Ok(atribuicao.selecionar(lotes))
    }
}

/// O caminho do PDF e o tamanho do lote, ja conferidos. O trecho so e conferido ao ler o PDF,
/// quando o total de paginas e conhecido; sem trecho, vale o documento inteiro.
#[derive(Debug, Clone)]
pub struct Pedido {
    pub caminho: String,
    pub folhas_por_lote: usize,
    pub intervalo: Option<IntervaloPaginas>,
    pub parte: Option<ParteDoEnvio>,
}

impl Pedido {
    /// Numero vindo de fora: zero vira recusa e o que passa do limite vai ao limite.
    pub fn novo(
        caminho: String,
        folhas_por_lote: usize,
        intervalo: Option<IntervaloPaginas>,
    ) -> Result<Self, Motivo> {
        if folhas_por_lote == 0 {
            return Err(Motivo::novo("pdf", "o lote deve ter ao menos uma folha"));
        }
        Ok(Self {
            caminho,
            folhas_por_lote: folhas_por_lote.min(LOTE_MAXIMO),
            intervalo,
            parte: None,
        })
    }

    /// O mesmo pedido, restrito aos lotes desta impressora na divisao.
    pub fn com_parte(mut self, parte: Option<ParteDoEnvio>) -> Self {
        self.parte = parte;
        self
    }
}

fn motivo_do_pdf(erro: &documento::ErroDocumento) -> Motivo {
    Motivo::novo("pdf", format!("não foi possível preparar {}", erro.nome))
}

fn planejar(
    dimensoes: &[DimensoesPagina],
    intervalo: Option<IntervaloPaginas>,
    folhas_por_lote: usize,
    perfil: PerfilImpressora,
) -> Result<Vec<LoteDoFluxo>, Motivo> {
    let lotes = plano_lotes_do_intervalo(
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

/// O plano de cada impressora sai do perfil dela; so depois a divisao fica com os lotes que lhe cabem.
pub(crate) fn planejar_pedido(
    dimensoes: &[DimensoesPagina],
    pedido: &Pedido,
    perfil: PerfilImpressora,
) -> Result<Vec<LoteDoFluxo>, Motivo> {
    if pedido
        .parte
        .is_some_and(|parte| parte.paginas != dimensoes.len())
    {
        return Err(documento_mudou());
    }
    let lotes = planejar(dimensoes, pedido.intervalo, pedido.folhas_por_lote, perfil)?;
    match pedido.parte {
        Some(parte) => parte.separar(lotes),
        None => Ok(lotes),
    }
}

/// A impressora simulada le o mesmo PDF e o mesmo plano, mas nada vai a uma fila.
pub fn preparar_simulada(
    modelo: ModeloImpressora,
    pedido: &Pedido,
) -> Result<Preparo<SimGuiado>, Motivo> {
    let lido = leitura_pdf::ler_pdf(&pedido.caminho).map_err(|erro| motivo_do_pdf(&erro))?;
    let lotes = planejar_pedido(&lido.dimensoes, pedido, modelo.perfil())?;
    let backend = SimGuiado::novo(Mecanica::do_modelo(modelo), modelo.gesto().into());
    let condutor =
        Condutor::novo(backend, lotes, ESPERA_SIMULADA).map_err(|erro| motivo_do_job(&erro))?;
    Ok(Preparo {
        condutor,
        recolocar: Recolocar::da_instrucao(Instrucao::do_modelo(modelo)),
        vira_para_ler: fluxo::vira_para_ler(modelo),
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
    let lotes = planejar_pedido(&lido.dimensoes, pedido, entrada.perfil)?;
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
    let backend =
        plots_windows::BackendWindows::novo(fila, lido.origem, dimensoes, pasta.as_deref())
            .map_err(|erro| erro.para_motivo())?;
    let condutor =
        Condutor::novo(backend, lotes, fluxo::ESPERA_REAL).map_err(|erro| motivo_do_job(&erro))?;
    Ok(Preparo {
        condutor,
        recolocar: recolocar_do_perfil(entrada.modelo, entrada.gesto_texto.clone()),
        vira_para_ler: entrada.modelo.is_some_and(fluxo::vira_para_ler),
    })
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
#[cfg_attr(not(windows), allow(dead_code))]
fn recolocar_do_perfil(modelo: Option<ModeloImpressora>, texto: Option<String>) -> Recolocar {
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
fn motivo_da_recusa(nome: &str, recusa: plots_windows::RecusaDeFila) -> Motivo {
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
    pub assinado: bool,
    /// Na divisao, o que cabe a cada impressora; no espelho, fica vazio.
    pub divisao: Vec<ParteDoResumo>,
    /// Paginas do documento inteiro: o envio confere de novo antes de imprimir.
    pub paginas_do_documento: usize,
    /// De qual a qual pagina vai cada lote, com os numeros do documento.
    pub faixas: Vec<FaixaDoLote>,
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

/// Quem imprime quais lotes, numerados como no documento. `vira_para_ler` vazio quer dizer que o
/// perfil nao afirma: vale o que foi visto na calibracao.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParteDoResumo {
    pub impressora: String,
    pub lotes: Vec<usize>,
    pub vira_para_ler: Option<bool>,
}

/// Lotes e papeis nao dependem do perfil: o plano padrao diz quantos sao. Com um trecho, tudo aqui
/// conta so o trecho.
pub fn resumo_do_envio(
    caminho: &str,
    folhas: usize,
    intervalo: Option<IntervaloPaginas>,
) -> Result<ResumoDoEnvio, documento::ErroDocumento> {
    let lido = leitura_pdf::ler_pdf(caminho)?;
    let falha = || documento::falha_inesperada(caminho);
    let folhas = folhas.clamp(1, LOTE_MAXIMO);
    let lotes = plano_lotes_do_intervalo(
        &lido.dimensoes,
        intervalo,
        folhas,
        OrdemPassadas::default(),
        PerfilImpressora::default(),
    )
    .map_err(|_| falha())?;
    let pares_mistos =
        plots_core::pares_mistos_do_intervalo(&lido.dimensoes, intervalo).map_err(|_| falha())?;
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
        assinado: lido.assinado,
        divisao: Vec::new(),
        paginas_do_documento: lido.dimensoes.len(),
        faixas: faixas_dos_lotes(&lotes),
    })
}

#[cfg(test)]
#[path = "trabalhos_testes.rs"]
mod testes;
