//! Canal local entre o PlotS em modo de teste e o simulador.
//!
//! Cada conexão leva uma linha e recebe uma linha de volta. Quem conecta pode ser qualquer programa:
//! a mensagem tem tamanho limitado e formato estrito, e um cliente mudo ou lento não prende o
//! servidor. O modo manual, em que a pessoa escolhe o gesto, não passa por aqui.

use crate::nome::NomeVirtual;
use plotsim_physics::{BordaQueEntra, Entrada, FaceNaEntrada, Gesto};
use std::fmt;

/// Nome do canal que o PlotS procura.
pub const CAMINHO_DO_CANAL: &str = r"\\.\pipe\plotsim";
/// Maior mensagem aceita, em bytes.
pub const LIMITE_MENSAGEM: usize = 512;
/// Maior número de lote aceito: o PlotS não envia mais páginas que isso.
pub const LIMITE_LOTES: u32 = 1_000_000;
const CABECALHO: &str = "plotsim\t1\t";

/// Como a pessoa lê o maço, quando o programa diz.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LeituraPedida {
    ComoSaiu,
    Virando,
    SemInstrucao,
}

/// O que o PlotS publica.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Mensagem {
    /// Vai começar; quantos lotes este envio tem.
    Inicio { fila: String, lotes: u32 },
    /// A frente do lote saiu, e este é o gesto. O verso só vai depois da resposta.
    Gesto {
        fila: String,
        lote: u32,
        gesto: Gesto,
    },
    /// O último verso terminou.
    Fim {
        fila: String,
        lotes: u32,
        leitura: LeituraPedida,
    },
}

impl Mensagem {
    pub fn fila(&self) -> &str {
        match self {
            Self::Inicio { fila, .. } | Self::Gesto { fila, .. } | Self::Fim { fila, .. } => fila,
        }
    }
}

/// Por que uma mensagem foi recusada.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErroMensagem {
    Grande,
    NaoEhTexto,
    Formato,
    Fila,
    Numero,
    Campo(&'static str),
}

impl fmt::Display for ErroMensagem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grande => write!(f, "a mensagem passa de {LIMITE_MENSAGEM} bytes"),
            Self::NaoEhTexto => f.write_str("a mensagem não é texto"),
            Self::Formato => f.write_str("a mensagem não segue o formato combinado"),
            Self::Fila => f.write_str("a mensagem não é de uma fila do simulador"),
            Self::Numero => f.write_str("a mensagem tem um número fora do limite"),
            Self::Campo(campo) => write!(f, "o campo {campo} tem um valor desconhecido"),
        }
    }
}

impl std::error::Error for ErroMensagem {}

fn numero(texto: &str, minimo: u32) -> Result<u32, ErroMensagem> {
    // `parse` aceita sinal; aqui só entram dígitos.
    if texto.is_empty() || !texto.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ErroMensagem::Numero);
    }
    let valor: u32 = texto.parse().map_err(|_| ErroMensagem::Numero)?;
    if valor < minimo || valor > LIMITE_LOTES {
        return Err(ErroMensagem::Numero);
    }
    Ok(valor)
}

fn fila_valida(texto: &str) -> Result<String, ErroMensagem> {
    let nome = NomeVirtual::da_fila(texto).map_err(|_| ErroMensagem::Fila)?;
    Ok(nome.fila())
}

fn campo<'a>(parte: Option<&'a str>, chave: &str) -> Result<&'a str, ErroMensagem> {
    parte
        .and_then(|texto| texto.strip_prefix(chave))
        .and_then(|texto| texto.strip_prefix('='))
        .ok_or(ErroMensagem::Formato)
}

/// Lê uma mensagem. Tudo vem de fora: formato estrito, sem campo a mais nem a menos.
pub fn analisar(bytes: &[u8]) -> Result<Mensagem, ErroMensagem> {
    if bytes.len() > LIMITE_MENSAGEM {
        return Err(ErroMensagem::Grande);
    }
    let texto = std::str::from_utf8(bytes).map_err(|_| ErroMensagem::NaoEhTexto)?;
    let linha = texto.strip_suffix('\n').ok_or(ErroMensagem::Formato)?;
    let corpo = linha.strip_prefix(CABECALHO).ok_or(ErroMensagem::Formato)?;
    if corpo.chars().any(|c| c.is_control() && c != '\t') {
        return Err(ErroMensagem::Formato);
    }
    let mut partes = corpo.split('\t');
    let tipo = partes.next().ok_or(ErroMensagem::Formato)?;
    let fila = fila_valida(campo(partes.next(), "fila")?)?;
    let mensagem = match tipo {
        "inicio" => Mensagem::Inicio {
            fila,
            lotes: numero(campo(partes.next(), "lotes")?, 1)?,
        },
        "gesto" => {
            let lote = numero(campo(partes.next(), "lote")?, 1)?;
            let entrada = match campo(partes.next(), "entrada")? {
                "traseira" => Entrada::Traseira,
                "gaveta" => Entrada::Gaveta,
                "manual" => Entrada::Manual,
                _ => return Err(ErroMensagem::Campo("entrada")),
            };
            let face_impressa = match campo(partes.next(), "face_impressa")? {
                "para_frente" => FaceNaEntrada::ParaFrente,
                "para_tras" => FaceNaEntrada::ParaTras,
                "para_cima" => FaceNaEntrada::ParaCima,
                "para_baixo" => FaceNaEntrada::ParaBaixo,
                _ => return Err(ErroMensagem::Campo("face_impressa")),
            };
            let borda_que_entra = match campo(partes.next(), "borda_que_entra")? {
                "topo" => BordaQueEntra::Topo,
                "base" => BordaQueEntra::Base,
                _ => return Err(ErroMensagem::Campo("borda_que_entra")),
            };
            let inverter_ordem = match campo(partes.next(), "inverter_ordem")? {
                "sim" => true,
                "nao" => false,
                _ => return Err(ErroMensagem::Campo("inverter_ordem")),
            };
            Mensagem::Gesto {
                fila,
                lote,
                gesto: Gesto {
                    entrada,
                    face_impressa,
                    borda_que_entra,
                    inverter_ordem,
                },
            }
        }
        "fim" => {
            let lotes = numero(campo(partes.next(), "lotes")?, 1)?;
            let leitura = match campo(partes.next(), "leitura")? {
                "como_saiu" => LeituraPedida::ComoSaiu,
                "virando" => LeituraPedida::Virando,
                "sem_instrucao" => LeituraPedida::SemInstrucao,
                _ => return Err(ErroMensagem::Campo("leitura")),
            };
            Mensagem::Fim {
                fila,
                lotes,
                leitura,
            }
        }
        _ => return Err(ErroMensagem::Formato),
    };
    if partes.next().is_some() {
        return Err(ErroMensagem::Formato);
    }
    Ok(mensagem)
}

/// A linha de resposta, sem quebra de linha nem caractere de controle vindos do motivo.
pub fn linha_de_resposta(resposta: &Result<(), String>) -> Vec<u8> {
    let mut linha = match resposta {
        Ok(()) => "ok".to_string(),
        Err(motivo) => {
            let limpo: String = motivo
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .take(200)
                .collect();
            format!("erro\t{limpo}")
        }
    };
    linha.push('\n');
    linha.into_bytes()
}

#[cfg(windows)]
pub use servidor::{ErroCanal, Pedido, ServidorDoCanal};

#[cfg(windows)]
mod servidor {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};
    use windows::core::{Error, HRESULT, PCWSTR};
    use windows::Win32::Foundation::{
        CloseHandle, ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_CONNECTED, ERROR_PIPE_LISTENING,
        HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows::Win32::Storage::FileSystem::{
        ReadFile, WriteFile, FILE_FLAGS_AND_ATTRIBUTES, FILE_FLAG_FIRST_PIPE_INSTANCE,
        PIPE_ACCESS_DUPLEX,
    };
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_NOWAIT, PIPE_READMODE_BYTE,
        PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
    };

    /// Quantos clientes atendidos ao mesmo tempo.
    const INSTANCIAS: usize = 4;
    const INTERVALO: Duration = Duration::from_millis(5);
    /// Quanto um cliente tem para mandar a mensagem inteira, e para receber a resposta.
    const PRAZO_DO_CLIENTE: Duration = Duration::from_secs(2);
    /// Quanto o servidor segura um gesto à espera de quem o cumpre.
    const PRAZO_DA_RESPOSTA: Duration = Duration::from_secs(600);
    const TAMANHO_DO_BUFFER: u32 = 1024;

    /// Falhas ao abrir o canal.
    #[derive(Debug)]
    pub enum ErroCanal {
        /// Outro programa já é dono do canal.
        JaAberto,
        Sistema(String),
    }

    impl fmt::Display for ErroCanal {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::JaAberto => f.write_str("outro programa já abriu o canal do simulador"),
                Self::Sistema(detalhe) => {
                    write!(f, "não deu para abrir o canal do simulador: {detalhe}")
                }
            }
        }
    }

    impl std::error::Error for ErroCanal {}

    /// Uma mensagem válida, à espera de resposta.
    pub struct Pedido {
        pub mensagem: Mensagem,
        resposta: Sender<Result<(), String>>,
    }

    impl Pedido {
        /// Responde ao cliente. Se ele já foi embora, não há o que fazer.
        pub fn responder(self, resposta: Result<(), String>) {
            let _ = self.resposta.send(resposta);
        }
    }

    enum Estado {
        Escutando,
        Lendo {
            desde: Instant,
            lidos: Vec<u8>,
        },
        Esperando {
            desde: Instant,
            resposta: Receiver<Result<(), String>>,
        },
        Escrevendo {
            desde: Instant,
            bytes: Vec<u8>,
            escritos: usize,
        },
        /// A resposta foi escrita; desligar agora a apagaria antes de o cliente ler.
        Drenando {
            desde: Instant,
        },
    }

    struct Instancia {
        canal: HANDLE,
        estado: Estado,
    }

    impl Drop for Instancia {
        fn drop(&mut self) {
            // SAFETY: o identificador é desta instância e não é usado depois daqui.
            unsafe {
                let _ = DisconnectNamedPipe(self.canal);
                let _ = CloseHandle(self.canal);
            }
        }
    }

    // SAFETY: o identificador do canal pode ser usado de outra thread; só uma o usa de cada vez.
    unsafe impl Send for Instancia {}

    fn e_erro(erro: &Error, codigo: windows::Win32::Foundation::WIN32_ERROR) -> bool {
        erro.code() == HRESULT::from_win32(codigo.0)
    }

    fn abrir(caminho: &[u16], primeira: bool) -> Result<HANDLE, ErroCanal> {
        let modo: FILE_FLAGS_AND_ATTRIBUTES = if primeira {
            FILE_FLAGS_AND_ATTRIBUTES(PIPE_ACCESS_DUPLEX.0 | FILE_FLAG_FIRST_PIPE_INSTANCE.0)
        } else {
            PIPE_ACCESS_DUPLEX
        };
        let maximo = u32::try_from(INSTANCIAS).unwrap_or(1);
        // SAFETY: `caminho` termina em zero e vive durante a chamada.
        let canal = unsafe {
            CreateNamedPipeW(
                PCWSTR(caminho.as_ptr()),
                modo,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
                maximo,
                TAMANHO_DO_BUFFER,
                TAMANHO_DO_BUFFER,
                0,
                None,
            )
        };
        if canal == INVALID_HANDLE_VALUE {
            let erro = Error::from_thread();
            // Ao pedir a primeira instância, acesso negado ou canal ocupado: o nome já tem dono.
            if primeira && [5, 231].map(HRESULT::from_win32).contains(&erro.code()) {
                return Err(ErroCanal::JaAberto);
            }
            return Err(ErroCanal::Sistema(erro.to_string()));
        }
        Ok(canal)
    }

    impl Instancia {
        /// Volta a escutar. Se o sistema recusar, o próximo passo tenta de novo.
        fn recomecar(&mut self) {
            // SAFETY: o identificador é desta instância.
            let _ = unsafe { DisconnectNamedPipe(self.canal) };
            self.estado = Estado::Escutando;
        }

        fn responder(&mut self, resposta: &Result<(), String>) {
            self.estado = Estado::Escrevendo {
                desde: Instant::now(),
                bytes: linha_de_resposta(resposta),
                escritos: 0,
            };
        }

        fn passo(&mut self, destino: &Sender<Pedido>) {
            match &mut self.estado {
                Estado::Escutando => {
                    // SAFETY: sem bloqueio, porque o canal foi aberto sem espera.
                    let conectou = match unsafe { ConnectNamedPipe(self.canal, None) } {
                        Ok(()) => true,
                        Err(erro) if e_erro(&erro, ERROR_PIPE_CONNECTED) => true,
                        Err(erro) if e_erro(&erro, ERROR_PIPE_LISTENING) => false,
                        // O cliente entrou e saiu antes de ser atendido.
                        Err(_) => {
                            self.recomecar();
                            false
                        }
                    };
                    if conectou {
                        self.estado = Estado::Lendo {
                            desde: Instant::now(),
                            lidos: Vec::new(),
                        };
                    }
                }
                Estado::Lendo { desde, lidos } => {
                    let desde = *desde;
                    let mut pedaco = [0u8; 256];
                    let mut quantos = 0u32;
                    // SAFETY: o buffer vive durante a chamada; o canal não bloqueia.
                    let leitura = unsafe {
                        ReadFile(self.canal, Some(&mut pedaco), Some(&mut quantos), None)
                    };
                    match leitura {
                        Ok(()) => {
                            let novos = usize::try_from(quantos)
                                .ok()
                                .and_then(|n| pedaco.get(..n))
                                .unwrap_or(&[]);
                            lidos.extend_from_slice(novos);
                            if lidos.len() > LIMITE_MENSAGEM {
                                self.responder(&Err(ErroMensagem::Grande.to_string()));
                            } else if lidos.contains(&b'\n') {
                                let bytes = std::mem::take(lidos);
                                self.entregar(&bytes, destino);
                            }
                        }
                        Err(erro) if e_erro(&erro, ERROR_NO_DATA) => {
                            if desde.elapsed() > PRAZO_DO_CLIENTE {
                                self.recomecar();
                            }
                        }
                        Err(_) => self.recomecar(),
                    }
                }
                Estado::Esperando { desde, resposta } => {
                    let tarde = desde.elapsed() > PRAZO_DA_RESPOSTA;
                    match resposta.try_recv() {
                        Ok(pronta) => self.responder(&pronta),
                        Err(TryRecvError::Disconnected) => {
                            self.responder(&Err("o simulador não respondeu".to_string()));
                        }
                        Err(TryRecvError::Empty) if tarde => {
                            self.responder(&Err("o simulador não respondeu a tempo".to_string()));
                        }
                        Err(TryRecvError::Empty) => {
                            if self.cliente_saiu() {
                                self.recomecar();
                            }
                        }
                    }
                }
                Estado::Escrevendo {
                    desde,
                    bytes,
                    escritos,
                } => {
                    let desde = *desde;
                    let resto = bytes.get(*escritos..).unwrap_or(&[]);
                    let mut quantos = 0u32;
                    // SAFETY: os bytes vivem durante a chamada; o canal não bloqueia.
                    let escrita =
                        unsafe { WriteFile(self.canal, Some(resto), Some(&mut quantos), None) };
                    match escrita {
                        Ok(()) => {
                            *escritos = escritos
                                .saturating_add(usize::try_from(quantos).unwrap_or(usize::MAX));
                            if *escritos >= bytes.len() {
                                self.estado = Estado::Drenando {
                                    desde: Instant::now(),
                                };
                            } else if desde.elapsed() > PRAZO_DO_CLIENTE {
                                self.recomecar();
                            }
                        }
                        Err(_) => self.recomecar(),
                    }
                }
                Estado::Drenando { desde } => {
                    let tarde = desde.elapsed() > PRAZO_DO_CLIENTE;
                    if tarde || self.cliente_saiu() {
                        self.recomecar();
                    }
                }
            }
        }

        /// Olha se o cliente fechou. O que ele ainda mandar é descartado: só cabe uma mensagem.
        fn cliente_saiu(&self) -> bool {
            let mut pedaco = [0u8; 64];
            let mut quantos = 0u32;
            // SAFETY: o buffer vive durante a chamada; o canal não bloqueia.
            match unsafe { ReadFile(self.canal, Some(&mut pedaco), Some(&mut quantos), None) } {
                Ok(()) => false,
                Err(erro) => e_erro(&erro, ERROR_BROKEN_PIPE),
            }
        }

        fn entregar(&mut self, bytes: &[u8], destino: &Sender<Pedido>) {
            let mensagem = match analisar(bytes) {
                Ok(mensagem) => mensagem,
                Err(erro) => {
                    self.responder(&Err(erro.to_string()));
                    return;
                }
            };
            let (resposta, chegada) = mpsc::channel();
            if destino.send(Pedido { mensagem, resposta }).is_err() {
                self.responder(&Err("o simulador está encerrando".to_string()));
                return;
            }
            self.estado = Estado::Esperando {
                desde: Instant::now(),
                resposta: chegada,
            };
        }
    }

    /// O servidor do canal, em uma thread própria que nunca bloqueia em um cliente.
    pub struct ServidorDoCanal {
        parar: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl ServidorDoCanal {
        /// Abre o canal e começa a atender. Cada mensagem válida chega por `destino`.
        pub fn iniciar(caminho: &str, destino: Sender<Pedido>) -> Result<Self, ErroCanal> {
            let largo: Vec<u16> = caminho.encode_utf16().chain(std::iter::once(0)).collect();
            let mut instancias = Vec::with_capacity(INSTANCIAS);
            for indice in 0..INSTANCIAS {
                instancias.push(Instancia {
                    canal: abrir(&largo, indice == 0)?,
                    estado: Estado::Escutando,
                });
            }
            let parar = Arc::new(AtomicBool::new(false));
            let parar_na_thread = Arc::clone(&parar);
            let thread = thread::Builder::new()
                .name("plotsim-canal".to_string())
                .spawn(move || {
                    let mut instancias = instancias;
                    while !parar_na_thread.load(Ordering::Relaxed) {
                        for instancia in &mut instancias {
                            instancia.passo(&destino);
                        }
                        thread::sleep(INTERVALO);
                    }
                })
                .map_err(|erro| ErroCanal::Sistema(erro.to_string()))?;
            Ok(Self {
                parar,
                thread: Some(thread),
            })
        }

        /// Para de atender e fecha o canal.
        pub fn parar(mut self) {
            self.encerrar();
        }

        fn encerrar(&mut self) {
            self.parar.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    impl Drop for ServidorDoCanal {
        fn drop(&mut self) {
            self.encerrar();
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const GESTO: &[u8] = "plotsim\t1\tgesto\tfila=PlotSim — Jato\tlote=3\tentrada=traseira\tface_impressa=para_tras\tborda_que_entra=base\tinverter_ordem=nao\n".as_bytes();

    #[test]
    fn le_as_tres_mensagens() {
        assert_eq!(
            analisar(GESTO),
            Ok(Mensagem::Gesto {
                fila: "PlotSim — Jato".to_string(),
                lote: 3,
                gesto: Gesto {
                    entrada: Entrada::Traseira,
                    face_impressa: FaceNaEntrada::ParaTras,
                    borda_que_entra: BordaQueEntra::Base,
                    inverter_ordem: false,
                }
            })
        );
        assert_eq!(
            analisar("plotsim\t1\tinicio\tfila=PlotSim — Jato\tlotes=2\n".as_bytes()),
            Ok(Mensagem::Inicio {
                fila: "PlotSim — Jato".to_string(),
                lotes: 2
            })
        );
        assert_eq!(
            analisar("plotsim\t1\tfim\tfila=PlotSim — Jato\tlotes=2\tleitura=virando\n".as_bytes()),
            Ok(Mensagem::Fim {
                fila: "PlotSim — Jato".to_string(),
                lotes: 2,
                leitura: LeituraPedida::Virando,
            })
        );
    }

    #[test]
    fn recusa_mensagem_grande_antes_de_olhar_o_conteudo() {
        assert_eq!(
            analisar(&vec![b'x'; LIMITE_MENSAGEM + 1]),
            Err(ErroMensagem::Grande)
        );
    }

    #[test]
    fn entrada_hostil_e_recusada_sem_travar() {
        let trocar = |de: &str, para: &str| {
            String::from_utf8(GESTO.to_vec())
                .unwrap()
                .replace(de, para)
                .into_bytes()
        };
        let casos: Vec<(Vec<u8>, ErroMensagem)> = vec![
            (Vec::new(), ErroMensagem::Formato),
            (vec![0xff, 0xfe, b'\n'], ErroMensagem::NaoEhTexto),
            (b"plotsim\t2\tinicio\n".to_vec(), ErroMensagem::Formato),
            (GESTO[..GESTO.len() - 1].to_vec(), ErroMensagem::Formato),
            (trocar("lote=3", "lote=0"), ErroMensagem::Numero),
            (trocar("lote=3", "lote=-1"), ErroMensagem::Numero),
            (trocar("lote=3", "lote=+1"), ErroMensagem::Numero),
            (trocar("lote=3", "lote="), ErroMensagem::Numero),
            (trocar("lote=3", "lote=99999999999"), ErroMensagem::Numero),
            (trocar("lote=3", "lote=1000001"), ErroMensagem::Numero),
            (
                trocar("fila=PlotSim — Jato", "fila=Impressora real"),
                ErroMensagem::Fila,
            ),
            (
                trocar("fila=PlotSim — Jato", "fila=PlotSim — a/b"),
                ErroMensagem::Fila,
            ),
            (
                trocar("entrada=traseira", "entrada=x"),
                ErroMensagem::Campo("entrada"),
            ),
            (
                trocar("face_impressa=para_tras", "face_impressa=x"),
                ErroMensagem::Campo("face_impressa"),
            ),
            (
                trocar("borda_que_entra=base", "borda_que_entra=x"),
                ErroMensagem::Campo("borda_que_entra"),
            ),
            (
                trocar("inverter_ordem=nao", "inverter_ordem=x"),
                ErroMensagem::Campo("inverter_ordem"),
            ),
            (
                trocar("inverter_ordem=nao", "inverter_ordem=nao\textra=1"),
                ErroMensagem::Formato,
            ),
            (trocar("lote=3\t", ""), ErroMensagem::Formato),
            (trocar("gesto", "outro"), ErroMensagem::Formato),
            (trocar("Jato", "Ja\u{7}to"), ErroMensagem::Formato),
        ];
        for (bytes, erro) in casos {
            assert_eq!(
                analisar(&bytes),
                Err(erro),
                "{:?}",
                String::from_utf8_lossy(&bytes)
            );
        }
    }

    #[test]
    fn a_resposta_nao_carrega_quebra_de_linha_do_motivo() {
        assert_eq!(linha_de_resposta(&Ok(())), b"ok\n");
        assert_eq!(
            linha_de_resposta(&Err("a\nb\tc".to_string())),
            b"erro\ta b c\n"
        );
        let longa = linha_de_resposta(&Err("x".repeat(10_000)));
        assert!(longa.len() < LIMITE_MENSAGEM);
    }

    #[cfg(windows)]
    mod no_sistema {
        use super::*;
        use crate::canal::{Pedido, ServidorDoCanal};
        use std::fs::{File, OpenOptions};
        use std::io::{Read, Write};
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::mpsc::{self, Receiver};
        use std::time::{Duration, Instant};

        static SEQUENCIA: AtomicU32 = AtomicU32::new(0);

        fn caminho_novo() -> String {
            let n = SEQUENCIA.fetch_add(1, Ordering::Relaxed);
            format!(r"\\.\pipe\plotsim-teste-{}-{n}", std::process::id())
        }

        fn servidor() -> (String, ServidorDoCanal, Receiver<Pedido>) {
            let caminho = caminho_novo();
            let (destino, chegada) = mpsc::channel();
            let servidor = ServidorDoCanal::iniciar(&caminho, destino).unwrap();
            (caminho, servidor, chegada)
        }

        fn conectar(caminho: &str) -> File {
            let inicio = Instant::now();
            loop {
                match OpenOptions::new().read(true).write(true).open(caminho) {
                    Ok(arquivo) => return arquivo,
                    Err(erro) if inicio.elapsed() > Duration::from_secs(5) => {
                        panic!("não conectou: {erro}")
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(10)),
                }
            }
        }

        fn ler_linha(canal: &mut File) -> String {
            let mut texto = Vec::new();
            let mut pedaco = [0u8; 64];
            while !texto.contains(&b'\n') {
                let n = canal.read(&mut pedaco).unwrap();
                assert!(n > 0, "o canal fechou antes da resposta");
                texto.extend_from_slice(&pedaco[..n]);
            }
            String::from_utf8(texto).unwrap()
        }

        #[test]
        fn sem_cliente_abre_e_fecha_sem_travar() {
            let (_, servidor, chegada) = servidor();
            assert!(chegada.recv_timeout(Duration::from_millis(50)).is_err());
            let inicio = Instant::now();
            servidor.parar();
            assert!(inicio.elapsed() < Duration::from_secs(2));
        }

        #[test]
        fn o_canal_tem_um_dono_so() {
            let (caminho, _servidor, _chegada) = servidor();
            let (destino, _outra) = mpsc::channel();
            assert!(matches!(
                ServidorDoCanal::iniciar(&caminho, destino),
                Err(crate::canal::ErroCanal::JaAberto)
            ));
        }

        #[test]
        fn mensagem_valida_chega_e_a_resposta_volta() {
            let (caminho, _servidor, chegada) = servidor();
            let mut canal = conectar(&caminho);
            canal.write_all(GESTO).unwrap();
            let pedido = chegada.recv_timeout(Duration::from_secs(5)).unwrap();
            assert_eq!(pedido.mensagem.fila(), "PlotSim — Jato");
            pedido.responder(Ok(()));
            assert_eq!(ler_linha(&mut canal), "ok\n");
        }

        #[test]
        fn a_recusa_do_simulador_volta_ao_cliente() {
            let (caminho, _servidor, chegada) = servidor();
            let mut canal = conectar(&caminho);
            canal.write_all(GESTO).unwrap();
            let pedido = chegada.recv_timeout(Duration::from_secs(5)).unwrap();
            pedido.responder(Err("lote fora de ordem".to_string()));
            assert_eq!(ler_linha(&mut canal), "erro\tlote fora de ordem\n");
        }

        #[test]
        fn lixo_recebe_erro_e_nao_chega_ao_simulador() {
            let (caminho, _servidor, chegada) = servidor();
            let mut canal = conectar(&caminho);
            canal.write_all(b"oi, tudo bem?\n").unwrap();
            assert!(ler_linha(&mut canal).starts_with("erro\t"));
            assert!(chegada.recv_timeout(Duration::from_millis(100)).is_err());
        }

        #[test]
        fn mensagem_sem_fim_e_cortada_no_limite() {
            let (caminho, _servidor, chegada) = servidor();
            let mut canal = conectar(&caminho);
            canal.write_all(&vec![b'x'; LIMITE_MENSAGEM + 300]).unwrap();
            assert!(ler_linha(&mut canal).contains("passa de"));
            assert!(chegada.recv_timeout(Duration::from_millis(100)).is_err());
        }

        #[test]
        fn cliente_mudo_nao_prende_o_canal() {
            let (caminho, _servidor, chegada) = servidor();
            let mudos: Vec<File> = (0..3).map(|_| conectar(&caminho)).collect();
            let mut canal = conectar(&caminho);
            canal.write_all(GESTO).unwrap();
            let pedido = chegada.recv_timeout(Duration::from_secs(5)).unwrap();
            pedido.responder(Ok(()));
            assert_eq!(ler_linha(&mut canal), "ok\n");
            drop(mudos);
        }

        #[test]
        fn depois_de_um_cliente_que_some_o_proximo_e_atendido() {
            let (caminho, _servidor, chegada) = servidor();
            for _ in 0..8 {
                let mut canal = conectar(&caminho);
                canal.write_all(GESTO).unwrap();
                let pedido = chegada.recv_timeout(Duration::from_secs(5)).unwrap();
                drop(canal);
                pedido.responder(Ok(()));
            }
            let mut canal = conectar(&caminho);
            canal.write_all(GESTO).unwrap();
            let pedido = chegada.recv_timeout(Duration::from_secs(5)).unwrap();
            pedido.responder(Ok(()));
            assert_eq!(ler_linha(&mut canal), "ok\n");
        }

        #[test]
        fn simulador_encerrado_vira_erro_para_o_cliente() {
            let (caminho, _servidor, chegada) = servidor();
            drop(chegada);
            let mut canal = conectar(&caminho);
            canal.write_all(GESTO).unwrap();
            assert!(ler_linha(&mut canal).starts_with("erro\t"));
        }
    }
}
