//! Recepção dos trabalhos de uma fila virtual.
//!
//! A porta grava todo trabalho no mesmo arquivo, e o trabalho seguinte o apaga ao começar. Por isso
//! o conteúdo é guardado na memória assim que o arquivo fica completo, ainda com o trabalho na
//! fila, e vai para o histórico quando o spooler solta o trabalho. O dono de cada arquivo novo sai
//! do estado que o spooler informa. Trabalho que sumir sem arquivo vira evento próprio: nada se
//! perde em silêncio.

use crate::leitor::{ler_arquivo, ErroLeitura, LIMITE_BYTES_PDF};
use crate::registro::gravar_atomico;
use crate::spooler::{ErroSpooler, Spooler, TrabalhoNaFila};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::VecDeque;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

/// De quanto em quanto tempo olhar o spooler, mesmo sem novidade na pasta.
const INTERVALO: Duration = Duration::from_millis(100);
/// Quantos bytes do fim do arquivo procurar pelo marcador de fim do PDF.
const CAUDA_DO_PDF: usize = 1024;
const MARCADOR_FIM: &[u8] = b"%%EOF";
/// Bits de `JOB_STATUS_*`: erro, apagando e apagado.
const STATUS_DE_FALHA: u32 = 0x0000_0002 | 0x0000_0004 | 0x0000_0100;
const STATUS_APAGANDO: u32 = 0x0000_0004;
const STATUS_IMPRIMINDO: u32 = 0x0000_0010;
/// Pausado, em erro ou bloqueado: não está gravando nada agora.
const STATUS_PARADO: u32 = 0x0000_0001 | 0x0000_0002 | 0x0000_0200;
/// Impresso ou completo.
const STATUS_CONCLUIDO: u32 = 0x0000_0080 | 0x0000_1000;
const STATUS_RETIDO: u32 = 0x0000_2000;
/// Maior soma de bytes guardados na memória: um trabalho preso não pode crescer sem fim.
const LIMITE_RETIDO: u64 = 1024 * 1024 * 1024;
/// Maior número de arquivos olhados ao procurar o próximo número do histórico.
const LIMITE_HISTORICO: usize = 1_000_000;

/// Falhas que impedem a recepção de continuar ou de começar.
#[derive(Debug)]
pub enum ErroRecepcao {
    Spooler(ErroSpooler),
    Historico(PathBuf, io::Error),
    HistoricoCheio,
    Observador(String),
}

impl fmt::Display for ErroRecepcao {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spooler(erro) => erro.fmt(f),
            Self::Historico(caminho, erro) => {
                write!(f, "não deu para usar {}: {erro}", caminho.display())
            }
            Self::HistoricoCheio => f.write_str("o histórico da fila está cheio"),
            Self::Observador(detalhe) => {
                write!(f, "não deu para observar a pasta da porta: {detalhe}")
            }
        }
    }
}

impl std::error::Error for ErroRecepcao {}

impl From<ErroSpooler> for ErroRecepcao {
    fn from(erro: ErroSpooler) -> Self {
        Self::Spooler(erro)
    }
}

/// Um trabalho guardado no histórico.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrabalhoRecebido {
    /// Número do trabalho no spooler. `None` quando o arquivo apareceu sem o trabalho ser visto.
    pub id: Option<u32>,
    pub documento: String,
    pub arquivo: PathBuf,
    pub bytes: u64,
}

/// O que aconteceu em uma fila.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Evento {
    Recebido(TrabalhoRecebido),
    /// O spooler marcou erro ou cancelamento e o trabalho saiu sem arquivo.
    Falhou {
        id: u32,
        documento: String,
    },
    /// O trabalho saiu da fila e o arquivo dele não chegou a ser visto completo.
    SemArquivo {
        id: u32,
        documento: String,
    },
    /// O arquivo da porta passa do limite aceito e não foi guardado.
    ArquivoRecusado {
        bytes: u64,
    },
    /// A leitura da fila falhou agora; a recepção continua tentando.
    Aviso(String),
}

/// Identidade do conteúdo do arquivo: muda quando o sistema o regrava.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Marca {
    tamanho: u64,
    modificado: Option<SystemTime>,
}

fn marca_de(caminho: &Path) -> Option<Marca> {
    let dados = fs::metadata(caminho).ok().filter(fs::Metadata::is_file)?;
    Some(Marca {
        tamanho: dados.len(),
        modificado: dados.modified().ok(),
    })
}

/// Um trabalho visto na fila, com o conteúdo dele quando já chegou.
struct Visto {
    id: u32,
    documento: String,
    status: u32,
    conteudo: Option<Vec<u8>>,
}

/// O spooler diz que a saída deste trabalho acabou.
fn encerrado(status: u32) -> bool {
    status & STATUS_CONCLUIDO != 0
        || (status & STATUS_RETIDO != 0 && status & STATUS_IMPRIMINDO == 0)
}

/// O último status de quem saiu da fila permite que o arquivo seja dele. Retido e apagando é o fim
/// normal de um trabalho bem-sucedido; apagando sem retido é trabalho tirado da fila.
fn saiu_sem_falha(status: u32) -> bool {
    let falha = if status & STATUS_RETIDO != 0 {
        STATUS_DE_FALHA & !STATUS_APAGANDO
    } else {
        STATUS_DE_FALHA
    };
    status & (falha | STATUS_PARADO) == 0
}

fn tamanho_de(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).unwrap_or(u64::MAX)
}

/// O PDF que a porta grava termina com o marcador. Antes disso, ele ainda está sendo escrito.
fn pdf_completo(bytes: &[u8]) -> bool {
    let inicio = bytes.len().saturating_sub(CAUDA_DO_PDF);
    bytes
        .get(inicio..)
        .is_some_and(|cauda| cauda.windows(MARCADOR_FIM.len()).any(|j| j == MARCADOR_FIM))
}

/// Acompanha uma fila, um passo por vez. Quem chama decide o ritmo.
pub struct Recepcao<S> {
    spooler: S,
    fila: String,
    porta: PathBuf,
    historico: PathBuf,
    limite: u64,
    /// Maior soma de conteúdos guardados na memória à espera de os trabalhos saírem da fila.
    limite_retido: u64,
    /// Na ordem em que a fila vai atendê-los.
    vistos: VecDeque<Visto>,
    /// Última versão do arquivo já tratada. O que estava lá antes de começar não é um trabalho.
    tratada: Option<Marca>,
    /// Última versão lida sem o marcador de fim: só vale ler de novo quando a marca mudar.
    sem_marcador: Option<Marca>,
    proximo: u64,
}

impl<S: Spooler> Recepcao<S> {
    /// Começa a acompanhar. O arquivo que já existir na porta fica de fora.
    pub fn nova(
        spooler: S,
        fila: String,
        porta: PathBuf,
        historico: PathBuf,
    ) -> Result<Self, ErroRecepcao> {
        fs::create_dir_all(&historico)
            .map_err(|erro| ErroRecepcao::Historico(historico.clone(), erro))?;
        let proximo = proximo_numero(&historico)?;
        let tratada = marca_de(&porta);
        Ok(Self {
            spooler,
            fila,
            porta,
            historico,
            limite: LIMITE_BYTES_PDF,
            limite_retido: LIMITE_RETIDO,
            vistos: VecDeque::new(),
            tratada,
            sem_marcador: None,
            proximo,
        })
    }

    #[cfg(test)]
    fn com_limite(mut self, limite: u64) -> Self {
        self.limite = limite;
        self
    }

    #[cfg(test)]
    fn com_limite_retido(mut self, limite: u64) -> Self {
        self.limite_retido = limite;
        self
    }

    pub fn porta(&self) -> &Path {
        &self.porta
    }

    /// Um passo: lê a fila, olha o arquivo e encerra os trabalhos que saíram.
    pub fn passo(&mut self) -> Result<Vec<Evento>, ErroRecepcao> {
        let trabalhos = self.spooler.trabalhos(&self.fila)?;
        self.acompanhar(&trabalhos);
        let mut eventos = Vec::new();
        self.olhar_arquivo(&trabalhos, &mut eventos)?;
        self.encerrar_os_que_sairam(&trabalhos, &mut eventos)?;
        Ok(eventos)
    }

    /// Há trabalho em andamento?
    pub fn ocupada(&self) -> bool {
        !self.vistos.is_empty()
    }

    fn acompanhar(&mut self, trabalhos: &[TrabalhoNaFila]) {
        for trabalho in trabalhos {
            match self.vistos.iter_mut().find(|visto| visto.id == trabalho.id) {
                Some(visto) => {
                    visto.status = trabalho.status;
                    // O sistema só troca o nome provisório depois que o trabalho já entrou.
                    visto.documento.clone_from(&trabalho.documento);
                }
                None => self.vistos.push_back(Visto {
                    id: trabalho.id,
                    documento: trabalho.documento.clone(),
                    status: trabalho.status,
                    conteudo: None,
                }),
            }
        }
    }

    /// Guarda o arquivo na memória se for uma versão nova e completa. Arquivo preso, vazio ou pela
    /// metade fica para o próximo passo.
    fn olhar_arquivo(
        &mut self,
        trabalhos: &[TrabalhoNaFila],
        eventos: &mut Vec<Evento>,
    ) -> Result<(), ErroRecepcao> {
        let Some(marca) = marca_de(&self.porta) else {
            return Ok(());
        };
        if self.tratada == Some(marca) || self.sem_marcador == Some(marca) || marca.tamanho == 0 {
            return Ok(());
        }
        let bytes = match ler_arquivo(&self.porta, self.limite) {
            Ok(bytes) => bytes,
            Err(ErroLeitura::Grande { bytes }) => {
                self.tratada = Some(marca);
                eventos.push(Evento::ArquivoRecusado { bytes });
                return Ok(());
            }
            Err(_) => return Ok(()),
        };
        if marca_de(&self.porta) != Some(marca) {
            return Ok(());
        }
        if !pdf_completo(&bytes) {
            self.sem_marcador = Some(marca);
            return Ok(());
        }
        self.tratada = Some(marca);
        let dono = self.dono_do_arquivo(trabalhos);
        let retidos = self
            .vistos
            .iter()
            .enumerate()
            .filter(|(indice, _)| Some(*indice) != dono)
            .filter_map(|(_, visto)| visto.conteudo.as_ref())
            .fold(0u64, |soma, outro| soma.saturating_add(tamanho_de(outro)));
        let novos = tamanho_de(&bytes);
        if dono.is_some() && retidos.saturating_add(novos) > self.limite_retido {
            eventos.push(Evento::ArquivoRecusado { bytes: novos });
            return Ok(());
        }
        match dono.and_then(|indice| self.vistos.get_mut(indice)) {
            Some(visto) => visto.conteudo = Some(bytes),
            None => {
                let recebido = self.guardar(None, String::new(), &bytes)?;
                eventos.push(Evento::Recebido(recebido));
            }
        }
        Ok(())
    }

    /// De quem é um arquivo novo: de quem o spooler diz que está imprimindo; senão, do primeiro que
    /// já saiu da fila sem arquivo e sem falha, porque a fila anda antes de o passo seguinte olhar e
    /// quem espera a vez ainda não gravou; senão, do primeiro presente que possa estar gravando; só
    /// então, de qualquer um sem arquivo.
    fn dono_do_arquivo(&self, trabalhos: &[TrabalhoNaFila]) -> Option<usize> {
        let presente = |visto: &Visto| trabalhos.iter().any(|trabalho| trabalho.id == visto.id);
        self.vistos
            .iter()
            .position(|visto| presente(visto) && visto.status & STATUS_IMPRIMINDO != 0)
            .or_else(|| {
                self.vistos.iter().position(|visto| {
                    !presente(visto) && visto.conteudo.is_none() && saiu_sem_falha(visto.status)
                })
            })
            .or_else(|| {
                self.vistos.iter().position(|visto| {
                    presente(visto)
                        && visto.status & STATUS_PARADO == 0
                        && !(encerrado(visto.status) && visto.conteudo.is_some())
                })
            })
            .or_else(|| {
                self.vistos
                    .iter()
                    .position(|visto| visto.conteudo.is_none())
            })
    }

    fn encerrar_os_que_sairam(
        &mut self,
        trabalhos: &[TrabalhoNaFila],
        eventos: &mut Vec<Evento>,
    ) -> Result<(), ErroRecepcao> {
        let mut restantes = VecDeque::with_capacity(self.vistos.len());
        let vistos = std::mem::take(&mut self.vistos);
        let mut falha = None;
        for visto in vistos {
            if trabalhos.iter().any(|trabalho| trabalho.id == visto.id) || falha.is_some() {
                restantes.push_back(visto);
                continue;
            }
            match self.encerrar(&visto) {
                Ok(evento) => eventos.push(evento),
                Err(erro) => {
                    // O conteúdo continua na memória e o próximo passo tenta gravar de novo.
                    restantes.push_back(visto);
                    falha = Some(erro);
                }
            }
        }
        self.vistos = restantes;
        match falha {
            Some(erro) => Err(erro),
            None => Ok(()),
        }
    }

    fn encerrar(&mut self, visto: &Visto) -> Result<Evento, ErroRecepcao> {
        if let Some(bytes) = &visto.conteudo {
            let recebido = self.guardar(Some(visto.id), visto.documento.clone(), bytes)?;
            return Ok(Evento::Recebido(recebido));
        }
        let documento = visto.documento.clone();
        Ok(if saiu_sem_falha(visto.status) {
            Evento::SemArquivo {
                id: visto.id,
                documento,
            }
        } else {
            Evento::Falhou {
                id: visto.id,
                documento,
            }
        })
    }

    /// Grava no histórico com um número novo. Um arquivo que já exista nunca é trocado.
    fn guardar(
        &mut self,
        id: Option<u32>,
        documento: String,
        bytes: &[u8],
    ) -> Result<TrabalhoRecebido, ErroRecepcao> {
        let mut numero = self.proximo;
        let arquivo = loop {
            let candidato = self.historico.join(format!("{numero:08}.pdf"));
            if !candidato.exists() {
                break candidato;
            }
            numero = numero.checked_add(1).ok_or(ErroRecepcao::HistoricoCheio)?;
        };
        gravar_atomico(&arquivo, bytes)
            .map_err(|erro| ErroRecepcao::Historico(arquivo.clone(), erro))?;
        self.proximo = numero.checked_add(1).ok_or(ErroRecepcao::HistoricoCheio)?;
        Ok(TrabalhoRecebido {
            id,
            documento,
            arquivo,
            bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        })
    }
}

/// Próximo número livre, olhando o que já está na pasta. Nomes estranhos são ignorados.
fn proximo_numero(historico: &Path) -> Result<u64, ErroRecepcao> {
    let leitura = fs::read_dir(historico)
        .map_err(|erro| ErroRecepcao::Historico(historico.to_path_buf(), erro))?;
    let mut maior = 0u64;
    for (lidos, entrada) in leitura.enumerate() {
        if lidos >= LIMITE_HISTORICO {
            return Err(ErroRecepcao::HistoricoCheio);
        }
        let nome = entrada
            .map_err(|erro| ErroRecepcao::Historico(historico.to_path_buf(), erro))?
            .file_name();
        let numero = nome
            .to_str()
            .and_then(|texto| texto.strip_suffix(".pdf"))
            .and_then(|texto| texto.parse::<u64>().ok());
        if let Some(numero) = numero {
            maior = maior.max(numero);
        }
    }
    maior.checked_add(1).ok_or(ErroRecepcao::HistoricoCheio)
}

/// Roda uma recepção em uma thread própria até `parar`. A pasta da porta é observada para acordar
/// na hora em que o arquivo muda; o spooler é consultado também sem novidade, porque o fim de um
/// trabalho não aparece na pasta.
pub struct Observador {
    parar: Arc<AtomicBool>,
    acordar: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Observador {
    pub fn iniciar<S: Spooler + Send + 'static>(
        mut recepcao: Recepcao<S>,
        destino: Sender<Evento>,
    ) -> Result<Self, ErroRecepcao> {
        let pasta = recepcao
            .porta()
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| ErroRecepcao::Observador("a porta não tem pasta".to_string()))?;
        let (acordar, sinais) = mpsc::channel();
        let sinal = acordar.clone();
        let falha = |erro: notify::Error| ErroRecepcao::Observador(erro.to_string());
        let mut vigia: RecommendedWatcher =
            notify::recommended_watcher(move |evento: notify::Result<notify::Event>| {
                let muda_algo =
                    evento.is_ok_and(|evento| !matches!(evento.kind, EventKind::Access(_)));
                if muda_algo {
                    let _ = sinal.send(());
                }
            })
            .map_err(falha)?;
        vigia
            .watch(&pasta, RecursiveMode::NonRecursive)
            .map_err(falha)?;
        let parar = Arc::new(AtomicBool::new(false));
        let parar_na_thread = Arc::clone(&parar);
        let thread = thread::Builder::new()
            .name("plotsim-recepcao".to_string())
            .spawn(move || {
                // O vigia vive enquanto a thread viver.
                let _vigia = vigia;
                let mut ultimo_aviso = None;
                while !parar_na_thread.load(Ordering::Relaxed) {
                    match recepcao.passo() {
                        Ok(eventos) => {
                            ultimo_aviso = None;
                            for evento in eventos {
                                if destino.send(evento).is_err() {
                                    return;
                                }
                            }
                        }
                        Err(erro) => {
                            // O mesmo problema só avisa uma vez até passar.
                            let texto = erro.to_string();
                            if ultimo_aviso.as_ref() != Some(&texto) {
                                if destino.send(Evento::Aviso(texto.clone())).is_err() {
                                    return;
                                }
                                ultimo_aviso = Some(texto);
                            }
                        }
                    }
                    match sinais.recv_timeout(INTERVALO) {
                        Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                    while sinais.try_recv().is_ok() {}
                }
            })
            .map_err(|erro| ErroRecepcao::Observador(erro.to_string()))?;
        Ok(Self {
            parar,
            acordar,
            thread: Some(thread),
        })
    }

    /// Pede para parar e espera a thread acabar.
    pub fn parar(mut self) {
        self.encerrar();
    }

    fn encerrar(&mut self) {
        self.parar.store(true, Ordering::Relaxed);
        let _ = self.acordar.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Observador {
    fn drop(&mut self) {
        self.encerrar();
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::testes_comuns::PastaTemporaria;
    use std::sync::Mutex;
    use std::time::Instant;

    const IMPRIMINDO: u32 = 0x10;
    const APAGANDO: u32 = 0x04;
    const PAUSADO: u32 = 0x01;
    const ERRO: u32 = 0x02;
    const RETIDO: u32 = 0x2000;

    /// Spooler com a lista que o teste mandar, ou uma falha.
    #[derive(Clone)]
    struct SpoolerFalso {
        lista: Arc<Mutex<Result<Vec<TrabalhoNaFila>, ErroSpooler>>>,
    }

    impl SpoolerFalso {
        fn novo() -> Self {
            Self {
                lista: Arc::new(Mutex::new(Ok(Vec::new()))),
            }
        }

        fn com(&self, trabalhos: &[(u32, &str, u32)]) {
            *self.lista.lock().unwrap() = Ok(trabalhos
                .iter()
                .map(|(id, documento, status)| TrabalhoNaFila {
                    id: *id,
                    documento: (*documento).to_string(),
                    status: *status,
                    paginas_totais: 1,
                    paginas_impressas: 0,
                })
                .collect());
        }

        fn falhando(&self) {
            *self.lista.lock().unwrap() = Err(ErroSpooler::Sistema {
                chamada: "EnumJobsW",
                codigo: 5,
            });
        }
    }

    impl Spooler for SpoolerFalso {
        fn trabalhos(&self, _fila: &str) -> Result<Vec<TrabalhoNaFila>, ErroSpooler> {
            self.lista.lock().unwrap().clone()
        }
    }

    struct Cena {
        pasta: PastaTemporaria,
        spooler: SpoolerFalso,
        porta: PathBuf,
        historico: PathBuf,
    }

    impl Cena {
        fn nova() -> Self {
            let pasta = PastaTemporaria::nova();
            let porta = pasta.caminho("porta/saida.pdf");
            fs::create_dir_all(porta.parent().unwrap()).unwrap();
            let historico = pasta.caminho("historico");
            Self {
                pasta,
                spooler: SpoolerFalso::novo(),
                porta,
                historico,
            }
        }

        fn recepcao(&self) -> Recepcao<SpoolerFalso> {
            Recepcao::nova(
                self.spooler.clone(),
                "fila".to_string(),
                self.porta.clone(),
                self.historico.clone(),
            )
            .unwrap()
        }

        /// Grava o arquivo, garantindo que a marca mude mesmo em sistema de arquivos grosseiro.
        fn gravar(&self, conteudo: &[u8]) {
            fs::write(&self.porta, conteudo).unwrap();
            let arquivo = fs::OpenOptions::new()
                .write(true)
                .open(&self.porta)
                .unwrap();
            let agora = SystemTime::now();
            arquivo.set_modified(agora).unwrap();
        }
    }

    fn pdf(marca: &str) -> Vec<u8> {
        format!("%PDF-1.7\n{marca}\n%%EOF\n").into_bytes()
    }

    fn recebido(evento: &Evento) -> &TrabalhoRecebido {
        match evento {
            Evento::Recebido(recebido) => recebido,
            outro => panic!("esperava recebido, veio {outro:?}"),
        }
    }

    #[test]
    fn trabalho_vai_para_o_historico_so_quando_sai_da_fila() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler.com(&[(1, "Doc", IMPRIMINDO)]);
        assert!(recepcao.passo().unwrap().is_empty());
        cena.gravar(&pdf("um"));
        assert!(recepcao.passo().unwrap().is_empty());
        assert_eq!(fs::read_dir(&cena.historico).unwrap().count(), 0);
        cena.spooler.com(&[]);
        let eventos = recepcao.passo().unwrap();
        assert_eq!(eventos.len(), 1);
        let item = recebido(&eventos[0]);
        assert_eq!(item.id, Some(1));
        assert_eq!(item.documento, "Doc");
        assert_eq!(fs::read(&item.arquivo).unwrap(), pdf("um"));
        assert!(!recepcao.ocupada());
    }

    #[test]
    fn dois_seguidos_com_o_segundo_apagando_o_arquivo_nao_perdem_nada() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler.com(&[(1, "A", IMPRIMINDO), (2, "B", 0)]);
        recepcao.passo().unwrap();
        cena.gravar(&pdf("primeiro"));
        recepcao.passo().unwrap();
        // O primeiro saiu e o segundo já apagou o arquivo, tudo entre dois passos.
        cena.spooler.com(&[(2, "B", IMPRIMINDO)]);
        cena.gravar(b"");
        let primeiros = recepcao.passo().unwrap();
        assert_eq!(
            fs::read(&recebido(&primeiros[0]).arquivo).unwrap(),
            pdf("primeiro")
        );
        cena.gravar(&pdf("segundo"));
        recepcao.passo().unwrap();
        cena.spooler.com(&[]);
        let segundos = recepcao.passo().unwrap();
        let item = recebido(&segundos[0]);
        assert_eq!(item.id, Some(2));
        assert_eq!(fs::read(&item.arquivo).unwrap(), pdf("segundo"));
        assert_ne!(recebido(&primeiros[0]).arquivo, item.arquivo);
    }

    #[test]
    fn sem_ninguem_presente_o_arquivo_fica_com_o_que_acabou_de_sair() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler.com(&[(1, "A", IMPRIMINDO)]);
        recepcao.passo().unwrap();
        cena.gravar(&pdf("primeiro"));
        cena.spooler.com(&[]);
        let eventos = recepcao.passo().unwrap();
        let item = recebido(&eventos[0]);
        assert_eq!(item.id, Some(1));
        assert_eq!(fs::read(&item.arquivo).unwrap(), pdf("primeiro"));
        assert!(!recepcao.ocupada());
    }

    #[test]
    fn arquivo_novo_vai_para_o_que_imprime_mesmo_com_outro_pausado_ou_em_erro_na_frente() {
        for parado in [PAUSADO, ERRO] {
            let cena = Cena::nova();
            let mut recepcao = cena.recepcao();
            cena.spooler.com(&[(1, "A", parado), (2, "B", IMPRIMINDO)]);
            recepcao.passo().unwrap();
            cena.gravar(&pdf("de B"));
            recepcao.passo().unwrap();
            cena.spooler.com(&[(1, "A", parado)]);
            let eventos = recepcao.passo().unwrap();
            let item = recebido(&eventos[0]);
            assert_eq!(item.id, Some(2), "com A em {parado:#x}");
            assert_eq!(fs::read(&item.arquivo).unwrap(), pdf("de B"));
            assert!(recepcao.ocupada());
        }
    }

    #[test]
    fn retido_com_arquivo_nao_fica_com_o_arquivo_do_proximo() {
        for proximo in [IMPRIMINDO, 0] {
            let cena = Cena::nova();
            let mut recepcao = cena.recepcao();
            cena.spooler.com(&[(1, "A", IMPRIMINDO)]);
            cena.gravar(&pdf("de A"));
            recepcao.passo().unwrap();
            cena.spooler.com(&[(1, "A", RETIDO), (2, "B", proximo)]);
            cena.gravar(&pdf("de B, que veio depois"));
            recepcao.passo().unwrap();
            cena.spooler.com(&[]);
            let eventos = recepcao.passo().unwrap();
            let (primeiro, segundo) = (recebido(&eventos[0]), recebido(&eventos[1]));
            assert_eq!(primeiro.id, Some(1));
            assert_eq!(fs::read(&primeiro.arquivo).unwrap(), pdf("de A"));
            assert_eq!(segundo.id, Some(2));
            assert_eq!(
                fs::read(&segundo.arquivo).unwrap(),
                pdf("de B, que veio depois")
            );
        }
    }

    #[test]
    fn em_rajada_o_arquivo_fica_com_o_que_saiu_e_nao_com_o_proximo_que_espera() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler
            .com(&[(1, "A", RETIDO | IMPRIMINDO), (2, "B", 0)]);
        assert!(recepcao.passo().unwrap().is_empty());
        cena.spooler.com(&[(2, "B", 0)]);
        cena.gravar(&pdf("de A"));
        let eventos = recepcao.passo().unwrap();
        assert_eq!(eventos.len(), 1, "{eventos:?}");
        let item = recebido(&eventos[0]);
        assert_eq!(item.id, Some(1));
        assert_eq!(fs::read(&item.arquivo).unwrap(), pdf("de A"));
        assert!(recepcao.ocupada());
    }

    #[test]
    fn o_que_saiu_retido_e_apagando_ainda_leva_o_proprio_arquivo() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler
            .com(&[(1, "A", RETIDO | APAGANDO | IMPRIMINDO), (2, "B", 0)]);
        assert!(recepcao.passo().unwrap().is_empty());
        cena.spooler.com(&[(2, "B", 0)]);
        cena.gravar(&pdf("de A"));
        let eventos = recepcao.passo().unwrap();
        assert_eq!(eventos.len(), 1, "{eventos:?}");
        let item = recebido(&eventos[0]);
        assert_eq!(item.id, Some(1));
        assert_eq!(fs::read(&item.arquivo).unwrap(), pdf("de A"));
    }

    #[test]
    fn sem_arquivo_o_ultimo_status_separa_o_fim_normal_da_falha() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler
            .com(&[(1, "A", RETIDO | APAGANDO | IMPRIMINDO)]);
        assert!(recepcao.passo().unwrap().is_empty());
        cena.spooler.com(&[]);
        assert_eq!(
            recepcao.passo().unwrap(),
            vec![Evento::SemArquivo {
                id: 1,
                documento: "A".to_string()
            }]
        );
        for ultimo in [APAGANDO, ERRO] {
            let cena = Cena::nova();
            let mut recepcao = cena.recepcao();
            cena.spooler.com(&[(1, "A", ultimo)]);
            assert!(recepcao.passo().unwrap().is_empty());
            cena.spooler.com(&[]);
            assert_eq!(
                recepcao.passo().unwrap(),
                vec![Evento::Falhou {
                    id: 1,
                    documento: "A".to_string()
                }],
                "com A em {ultimo:#x}"
            );
        }
    }

    #[test]
    fn o_que_saiu_com_falha_ou_parado_nao_leva_o_arquivo_do_proximo() {
        for ultimo in [APAGANDO, ERRO, PAUSADO] {
            let cena = Cena::nova();
            let mut recepcao = cena.recepcao();
            cena.spooler.com(&[(1, "A", ultimo), (2, "B", 0)]);
            assert!(recepcao.passo().unwrap().is_empty());
            cena.spooler.com(&[(2, "B", 0)]);
            cena.gravar(&pdf("de B"));
            let eventos = recepcao.passo().unwrap();
            assert_eq!(eventos.len(), 1, "com A em {ultimo:#x}: {eventos:?}");
            assert!(
                !matches!(eventos[0], Evento::Recebido(_)),
                "com A em {ultimo:#x}: {eventos:?}"
            );
            cena.spooler.com(&[]);
            let eventos = recepcao.passo().unwrap();
            let item = recebido(&eventos[0]);
            assert_eq!(item.id, Some(2), "com A em {ultimo:#x}");
            assert_eq!(fs::read(&item.arquivo).unwrap(), pdf("de B"));
        }
    }

    #[test]
    fn com_consulta_atrasada_o_que_saiu_nao_leva_o_arquivo_do_que_imprime() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler.com(&[(1, "A", IMPRIMINDO)]);
        recepcao.passo().unwrap();
        cena.spooler.com(&[(2, "B", IMPRIMINDO)]);
        cena.gravar(&pdf("de B"));
        assert_eq!(
            recepcao.passo().unwrap(),
            vec![Evento::SemArquivo {
                id: 1,
                documento: "A".to_string()
            }]
        );
        cena.spooler.com(&[]);
        let eventos = recepcao.passo().unwrap();
        let item = recebido(&eventos[0]);
        assert_eq!(item.id, Some(2));
        assert_eq!(fs::read(&item.arquivo).unwrap(), pdf("de B"));
    }

    #[test]
    fn soma_retida_na_memoria_tem_limite() {
        let cena = Cena::nova();
        let tamanho = u64::try_from(pdf("um").len()).unwrap();
        let mut recepcao = cena.recepcao().com_limite_retido(tamanho + 1);
        cena.spooler.com(&[(1, "A", IMPRIMINDO), (2, "B", 0)]);
        cena.gravar(&pdf("um"));
        recepcao.passo().unwrap();
        // O próprio trabalho pode trocar a sua versão: só a soma dos outros conta.
        cena.gravar(&pdf("xx"));
        assert!(recepcao.passo().unwrap().is_empty());
        cena.spooler.com(&[(1, "A", RETIDO), (2, "B", IMPRIMINDO)]);
        cena.gravar(&pdf("dois"));
        let bytes = fs::metadata(&cena.porta).unwrap().len();
        assert_eq!(
            recepcao.passo().unwrap(),
            vec![Evento::ArquivoRecusado { bytes }]
        );
        cena.spooler.com(&[]);
        let eventos = recepcao.passo().unwrap();
        assert_eq!(fs::read(&recebido(&eventos[0]).arquivo).unwrap(), pdf("xx"));
        assert_eq!(
            eventos[1],
            Evento::SemArquivo {
                id: 2,
                documento: "B".to_string()
            }
        );
    }

    #[test]
    fn arquivo_sem_marcador_nao_e_relido_enquanto_a_marca_nao_muda() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler.com(&[(1, "A", IMPRIMINDO)]);
        cena.gravar(b"%PDF-1.7\n1234567890123");
        let quando = fs::metadata(&cena.porta).unwrap().modified().unwrap();
        recepcao.passo().unwrap();
        // Mesmo tamanho e mesma data: para quem só olha a marca, nada mudou.
        fs::write(&cena.porta, b"%PDF-1.7\n1234\n%%EOF\n12").unwrap();
        let arquivo = fs::OpenOptions::new()
            .write(true)
            .open(&cena.porta)
            .unwrap();
        arquivo.set_modified(quando).unwrap();
        drop(arquivo);
        recepcao.passo().unwrap();
        cena.spooler.com(&[]);
        assert!(matches!(
            recepcao.passo().unwrap()[0],
            Evento::SemArquivo { .. }
        ));
    }

    #[test]
    fn dois_que_somem_de_uma_vez_acusam_a_perda_em_vez_de_esconder() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler.com(&[(1, "A", IMPRIMINDO), (2, "B", 0)]);
        recepcao.passo().unwrap();
        cena.spooler.com(&[]);
        let eventos = recepcao.passo().unwrap();
        assert_eq!(
            eventos,
            vec![
                Evento::SemArquivo {
                    id: 1,
                    documento: "A".to_string()
                },
                Evento::SemArquivo {
                    id: 2,
                    documento: "B".to_string()
                },
            ]
        );
    }

    #[test]
    fn arquivo_pela_metade_espera_ficar_completo() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler.com(&[(1, "A", IMPRIMINDO)]);
        cena.gravar(b"%PDF-1.7\nmetade");
        recepcao.passo().unwrap();
        cena.gravar(&pdf("inteiro"));
        recepcao.passo().unwrap();
        cena.spooler.com(&[]);
        let eventos = recepcao.passo().unwrap();
        assert_eq!(
            fs::read(&recebido(&eventos[0]).arquivo).unwrap(),
            pdf("inteiro")
        );
    }

    #[test]
    fn arquivo_regravado_pelo_mesmo_trabalho_vale_a_ultima_versao() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler.com(&[(1, "A", IMPRIMINDO)]);
        cena.gravar(&pdf("primeira versao"));
        recepcao.passo().unwrap();
        cena.gravar(&pdf("segunda versao e maior"));
        recepcao.passo().unwrap();
        cena.spooler.com(&[]);
        let eventos = recepcao.passo().unwrap();
        assert_eq!(
            fs::read(&recebido(&eventos[0]).arquivo).unwrap(),
            pdf("segunda versao e maior")
        );
    }

    #[test]
    fn arquivo_que_ja_estava_na_porta_nao_vira_trabalho() {
        let cena = Cena::nova();
        cena.gravar(&pdf("velho"));
        let mut recepcao = cena.recepcao();
        assert!(recepcao.passo().unwrap().is_empty());
        assert_eq!(fs::read_dir(&cena.historico).unwrap().count(), 0);
    }

    #[test]
    fn arquivo_novo_sem_trabalho_visto_e_guardado_sem_numero_do_spooler() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.gravar(&pdf("rapido"));
        let eventos = recepcao.passo().unwrap();
        assert_eq!(recebido(&eventos[0]).id, None);
    }

    #[test]
    fn trabalho_com_erro_ou_cancelado_e_avisado_como_falha() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler.com(&[(1, "A", APAGANDO)]);
        recepcao.passo().unwrap();
        cena.spooler.com(&[]);
        assert_eq!(
            recepcao.passo().unwrap(),
            vec![Evento::Falhou {
                id: 1,
                documento: "A".to_string()
            }]
        );
    }

    #[test]
    fn arquivo_acima_do_limite_e_recusado_uma_vez_e_o_trabalho_aparece_sem_arquivo() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao().com_limite(4);
        cena.spooler.com(&[(1, "A", IMPRIMINDO)]);
        cena.gravar(&pdf("grande"));
        let bytes = fs::metadata(&cena.porta).unwrap().len();
        assert_eq!(
            recepcao.passo().unwrap(),
            vec![Evento::ArquivoRecusado { bytes }]
        );
        assert!(recepcao.passo().unwrap().is_empty());
        cena.spooler.com(&[]);
        assert!(matches!(
            recepcao.passo().unwrap()[0],
            Evento::SemArquivo { .. }
        ));
    }

    #[test]
    fn historico_continua_a_numeracao_ignora_nomes_estranhos_e_nao_sobrescreve() {
        let cena = Cena::nova();
        fs::create_dir_all(&cena.historico).unwrap();
        fs::write(cena.historico.join("00000007.pdf"), b"antigo").unwrap();
        for estranho in ["abc.pdf", "-1.pdf", "99999999999999999999999.pdf", "12.txt"] {
            fs::write(cena.historico.join(estranho), b"x").unwrap();
        }
        let mut recepcao = cena.recepcao();
        cena.gravar(&pdf("novo"));
        let eventos = recepcao.passo().unwrap();
        let item = recebido(&eventos[0]);
        assert!(item.arquivo.ends_with("00000008.pdf"));
        assert_eq!(
            fs::read(cena.historico.join("00000007.pdf")).unwrap(),
            b"antigo"
        );
    }

    #[test]
    fn numero_no_limite_do_historico_vira_erro_e_nao_estouro() {
        let cena = Cena::nova();
        fs::create_dir_all(&cena.historico).unwrap();
        fs::write(cena.historico.join(format!("{}.pdf", u64::MAX)), b"x").unwrap();
        assert!(matches!(
            Recepcao::nova(
                cena.spooler.clone(),
                "fila".to_string(),
                cena.porta.clone(),
                cena.historico.clone()
            ),
            Err(ErroRecepcao::HistoricoCheio)
        ));
    }

    #[test]
    fn falha_ao_gravar_no_historico_nao_perde_o_trabalho() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler.com(&[(1, "A", IMPRIMINDO)]);
        cena.gravar(&pdf("um"));
        recepcao.passo().unwrap();
        // Uma pasta no lugar do histórico faz a gravação falhar.
        fs::remove_dir_all(&cena.historico).unwrap();
        fs::write(&cena.historico, b"nao sou pasta").unwrap();
        cena.spooler.com(&[]);
        assert!(matches!(recepcao.passo(), Err(ErroRecepcao::Historico(..))));
        fs::remove_file(&cena.historico).unwrap();
        let eventos = recepcao.passo().unwrap();
        assert_eq!(fs::read(&recebido(&eventos[0]).arquivo).unwrap(), pdf("um"));
    }

    #[test]
    fn spooler_que_falha_vira_erro_e_nao_encerra_trabalho() {
        let cena = Cena::nova();
        let mut recepcao = cena.recepcao();
        cena.spooler.com(&[(1, "A", IMPRIMINDO)]);
        recepcao.passo().unwrap();
        cena.spooler.falhando();
        assert!(recepcao.passo().is_err());
        assert!(recepcao.ocupada());
    }

    #[test]
    fn pdf_completo_olha_so_o_fim_do_arquivo() {
        assert!(pdf_completo(b"%PDF %%EOF\r\n"));
        assert!(!pdf_completo(b""));
        assert!(!pdf_completo(b"%PDF sem fim"));
        let mut longo = b"%%EOF".to_vec();
        longo.extend(std::iter::repeat_n(b' ', CAUDA_DO_PDF + 1));
        assert!(!pdf_completo(&longo));
    }

    #[test]
    fn observador_entrega_dois_trabalhos_seguidos_e_para_quando_pedem() {
        let cena = Cena::nova();
        let (destino, eventos) = mpsc::channel();
        let observador = Observador::iniciar(cena.recepcao(), destino).unwrap();
        let esperar_arquivo_visto = || std::thread::sleep(INTERVALO.saturating_mul(4));
        for marca in ["um", "dois"] {
            cena.spooler.com(&[(marca.len() as u32, marca, IMPRIMINDO)]);
            esperar_arquivo_visto();
            cena.gravar(&pdf(marca));
            esperar_arquivo_visto();
            cena.spooler.com(&[]);
            let chegou = eventos.recv_timeout(Duration::from_secs(10)).unwrap();
            assert_eq!(fs::read(&recebido(&chegou).arquivo).unwrap(), pdf(marca));
        }
        let antes = Instant::now();
        observador.parar();
        assert!(antes.elapsed() < Duration::from_secs(10));
        assert!(eventos.recv_timeout(Duration::from_secs(1)).is_err());
        drop(cena.pasta);
    }

    #[test]
    fn observador_avisa_uma_vez_quando_o_spooler_falha() {
        let cena = Cena::nova();
        cena.spooler.falhando();
        let (destino, eventos) = mpsc::channel();
        let observador = Observador::iniciar(cena.recepcao(), destino).unwrap();
        assert!(matches!(
            eventos.recv_timeout(Duration::from_secs(10)).unwrap(),
            Evento::Aviso(_)
        ));
        std::thread::sleep(INTERVALO.saturating_mul(3));
        assert!(eventos.try_recv().is_err());
        observador.parar();
    }
}
