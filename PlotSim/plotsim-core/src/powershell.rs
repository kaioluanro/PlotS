//! Filas e portas pelo PowerShell. Os scripts são textos fixos: nome e caminho só chegam a eles
//! por variável de ambiente, e o shell nunca os interpreta como código.

use crate::sistema::{ErroSistema, FilaDoSistema, SistemaDeFilas};
use std::time::Duration;

const VARIAVEL_FILA: &str = "PLOTSIM_FILA";
const VARIAVEL_PORTA: &str = "PLOTSIM_PORTA";
const VARIAVEL_DRIVER: &str = "PLOTSIM_DRIVER";
/// Tempo máximo de cada comando.
const PRAZO: Duration = Duration::from_secs(60);
/// Maior trecho do texto do sistema guardado em um erro.
const LIMITE_MENSAGEM: usize = 500;
const MARCA_ERRO: &str = "PLOTSIM_ERRO|";

/// Início de todo script: qualquer erro para o script e sai em uma linha que dá para ler.
const INICIO: &str = "$ErrorActionPreference='Stop'\n\
[Console]::OutputEncoding=New-Object System.Text.UTF8Encoding($false)\n\
trap {\n\
  $h='0x{0:X8}' -f $_.Exception.HResult\n\
  $c=[string]$_.CategoryInfo.Category\n\
  $m=([string]$_.Exception.Message) -replace '\\s+',' '\n\
  [Console]::Error.WriteLine('PLOTSIM_ERRO|'+$c+'|'+$h+'|'+$m)\n\
  exit 3\n\
}\n";
const PORTA_EXISTE: &str = "if (Get-PrinterPort -Name $env:PLOTSIM_PORTA -ErrorAction SilentlyContinue) { 'sim' } else { 'nao' }\n";
const CRIAR_PORTA: &str = "Add-PrinterPort -Name $env:PLOTSIM_PORTA\n";
const CRIAR_FILA: &str =
    "Add-Printer -Name $env:PLOTSIM_FILA -DriverName $env:PLOTSIM_DRIVER -PortName $env:PLOTSIM_PORTA\n";
const REMOVER_FILA: &str = "Remove-Printer -Name $env:PLOTSIM_FILA\n";
const REMOVER_PORTA: &str = "Remove-PrinterPort -Name $env:PLOTSIM_PORTA\n";

/// Um comando pronto: texto fixo e valores à parte.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComandoDoShell {
    pub script: String,
    pub variaveis: Vec<(&'static str, String)>,
    pub prazo: Duration,
}

/// O que um comando devolveu.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resposta {
    pub sucesso: bool,
    pub saida: String,
    pub erro: String,
}

/// Quem roda o comando. Nos testes, um falso guarda o pedido e devolve o que o teste quiser.
pub trait ExecutorDoShell {
    fn executar(&self, comando: &ComandoDoShell) -> Result<Resposta, ErroSistema>;
}

/// Lê as filas pelo spooler, sem abrir um shell. Separado para os testes.
pub trait LeitorDeFilas {
    fn ler(&self) -> Result<Vec<FilaDoSistema>, ErroSistema>;
}

/// Lista as filas com a leitura já existente do programa principal.
pub struct LeitorDoSpooler;

impl LeitorDeFilas for LeitorDoSpooler {
    fn ler(&self) -> Result<Vec<FilaDoSistema>, ErroSistema> {
        let filas = plots_windows::listar_filas().map_err(|erro| match erro {
            plots_windows::ErroFilas::NaoSuportado => ErroSistema::NaoSuportado,
            outro => ErroSistema::Leitura(outro.to_string()),
        })?;
        Ok(filas
            .into_iter()
            .map(|fila| FilaDoSistema {
                nome: fila.nome,
                driver: fila.driver,
                porta: fila.porta,
            })
            .collect())
    }
}

/// Filas e portas por comandos do PowerShell.
pub struct PowerShell<E, L> {
    executor: E,
    leitor: L,
}

#[cfg(windows)]
impl PowerShell<ProcessoDoShell, LeitorDoSpooler> {
    /// O sistema de verdade.
    pub fn do_sistema() -> Self {
        Self::novo(ProcessoDoShell, LeitorDoSpooler)
    }
}

impl<E: ExecutorDoShell, L: LeitorDeFilas> PowerShell<E, L> {
    pub fn novo(executor: E, leitor: L) -> Self {
        Self { executor, leitor }
    }

    fn rodar(
        &self,
        script: &str,
        variaveis: Vec<(&'static str, String)>,
    ) -> Result<String, ErroSistema> {
        let comando = ComandoDoShell {
            script: format!("{INICIO}{script}"),
            variaveis,
            prazo: PRAZO,
        };
        let resposta = self.executor.executar(&comando)?;
        if resposta.sucesso {
            return Ok(resposta.saida);
        }
        Err(traduzir_erro(&resposta.erro))
    }
}

impl<E: ExecutorDoShell, L: LeitorDeFilas> SistemaDeFilas for PowerShell<E, L> {
    fn filas(&self) -> Result<Vec<FilaDoSistema>, ErroSistema> {
        self.leitor.ler()
    }

    fn porta_existe(&self, porta: &str) -> Result<bool, ErroSistema> {
        let saida = self.rodar(PORTA_EXISTE, vec![(VARIAVEL_PORTA, porta.to_string())])?;
        match saida.trim() {
            "sim" => Ok(true),
            "nao" => Ok(false),
            outra => Err(ErroSistema::Processo(format!(
                "resposta inesperada: {}",
                cortar(outra)
            ))),
        }
    }

    fn criar_porta(&self, porta: &str) -> Result<(), ErroSistema> {
        self.rodar(CRIAR_PORTA, vec![(VARIAVEL_PORTA, porta.to_string())])
            .map(|_| ())
    }

    fn criar_fila(&self, fila: &str, driver: &str, porta: &str) -> Result<(), ErroSistema> {
        self.rodar(
            CRIAR_FILA,
            vec![
                (VARIAVEL_FILA, fila.to_string()),
                (VARIAVEL_DRIVER, driver.to_string()),
                (VARIAVEL_PORTA, porta.to_string()),
            ],
        )
        .map(|_| ())
    }

    fn remover_fila(&self, fila: &str) -> Result<(), ErroSistema> {
        self.rodar(REMOVER_FILA, vec![(VARIAVEL_FILA, fila.to_string())])
            .map(|_| ())
    }

    fn remover_porta(&self, porta: &str) -> Result<(), ErroSistema> {
        self.rodar(REMOVER_PORTA, vec![(VARIAVEL_PORTA, porta.to_string())])
            .map(|_| ())
    }
}

fn cortar(texto: &str) -> String {
    texto.chars().take(LIMITE_MENSAGEM).collect()
}

/// Lê a linha de erro do script. A falta de permissão vem por categoria ou pelo código do sistema,
/// sem depender do idioma do Windows.
fn traduzir_erro(erro: &str) -> ErroSistema {
    let Some(linha) = erro
        .lines()
        .find_map(|linha| linha.strip_prefix(MARCA_ERRO))
    else {
        return ErroSistema::Processo(cortar(erro.trim()));
    };
    let mut partes = linha.splitn(3, '|');
    let categoria = partes.next().unwrap_or_default();
    let codigo = partes.next().unwrap_or_default();
    let mensagem = partes.next().unwrap_or_default();
    if categoria == "PermissionDenied" || codigo == "0x80070005" {
        return ErroSistema::PrecisaAdministrador;
    }
    ErroSistema::Recusado {
        codigo: format!("{categoria} {codigo}"),
        mensagem: cortar(mensagem),
    }
}

const ALFABETO: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn letra_base64(bits: u32) -> char {
    let indice = usize::try_from(bits & 63).unwrap_or(0);
    char::from(ALFABETO.get(indice).copied().unwrap_or(b'A'))
}

/// Texto em UTF-16 e base64, a forma de `-EncodedCommand`: o script não passa por aspas de ninguém.
pub(crate) fn script_codificado(script: &str) -> String {
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut texto = String::with_capacity(bytes.len().div_ceil(3).saturating_mul(4));
    for grupo in bytes.chunks(3) {
        let a = grupo.first().copied().unwrap_or(0);
        let b = grupo.get(1).copied().unwrap_or(0);
        let c = grupo.get(2).copied().unwrap_or(0);
        let bits = (u32::from(a) << 16) | (u32::from(b) << 8) | u32::from(c);
        texto.push(letra_base64(bits >> 18));
        texto.push(letra_base64(bits >> 12));
        texto.push(if grupo.len() > 1 {
            letra_base64(bits >> 6)
        } else {
            '='
        });
        texto.push(if grupo.len() > 2 {
            letra_base64(bits)
        } else {
            '='
        });
    }
    texto
}

#[cfg(windows)]
pub use processo::ProcessoDoShell;

#[cfg(windows)]
mod processo {
    use super::{script_codificado, ComandoDoShell, ErroSistema, ExecutorDoShell, Resposta};
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    /// Sem janela de console.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const PAUSA: Duration = Duration::from_millis(25);
    /// Maior saída aceita de um comando, em bytes.
    const LIMITE_SAIDA: u64 = 256 * 1024;

    /// Roda o PowerShell do Windows, pelo caminho absoluto: nunca o que estiver no `PATH`.
    pub struct ProcessoDoShell;

    fn caminho_do_shell() -> Result<PathBuf, ErroSistema> {
        let sistema = std::env::var_os("SystemRoot")
            .ok_or_else(|| ErroSistema::Processo("SystemRoot não está definida".to_string()))?;
        Ok(PathBuf::from(sistema)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe"))
    }

    fn ler_em_thread(
        fluxo: Option<impl Read + Send + 'static>,
    ) -> JoinHandle<Result<Vec<u8>, String>> {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(fluxo) = fluxo {
                fluxo
                    .take(LIMITE_SAIDA.saturating_add(1))
                    .read_to_end(&mut bytes)
                    .map_err(|erro| erro.to_string())?;
            }
            Ok(bytes)
        })
    }

    fn encerrar(filho: &mut Child) {
        // Encerra só o processo que este comando abriu.
        let _ = filho.kill();
        let _ = filho.wait();
    }

    fn colher(leitor: JoinHandle<Result<Vec<u8>, String>>) -> Result<String, ErroSistema> {
        let bytes = leitor
            .join()
            .map_err(|_| ErroSistema::Processo("a leitura da saída parou".to_string()))?
            .map_err(ErroSistema::Processo)?;
        if u64::try_from(bytes.len()).map_or(true, |lidos| lidos > LIMITE_SAIDA) {
            return Err(ErroSistema::Processo(
                "o comando devolveu texto demais".to_string(),
            ));
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    impl ExecutorDoShell for ProcessoDoShell {
        fn executar(&self, comando: &ComandoDoShell) -> Result<Resposta, ErroSistema> {
            let mut chamada = Command::new(caminho_do_shell()?);
            chamada
                .args([
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-EncodedCommand",
                ])
                .arg(script_codificado(&comando.script))
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(CREATE_NO_WINDOW);
            for (nome, valor) in &comando.variaveis {
                chamada.env(nome, valor);
            }
            let mut filho = chamada
                .spawn()
                .map_err(|erro| ErroSistema::Processo(erro.to_string()))?;
            let saida = ler_em_thread(filho.stdout.take());
            let erro = ler_em_thread(filho.stderr.take());
            let inicio = Instant::now();
            let status = loop {
                match filho.try_wait() {
                    Ok(Some(status)) => break status,
                    Ok(None) if inicio.elapsed() >= comando.prazo => {
                        encerrar(&mut filho);
                        return Err(ErroSistema::TempoEsgotado);
                    }
                    Ok(None) => thread::sleep(PAUSA),
                    Err(falha) => {
                        encerrar(&mut filho);
                        return Err(ErroSistema::Processo(falha.to_string()));
                    }
                }
            };
            Ok(Resposta {
                sucesso: status.success(),
                saida: colher(saida)?,
                erro: colher(erro)?,
            })
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct ExecutorFalso {
        pedidos: Mutex<Vec<ComandoDoShell>>,
        erro: Option<String>,
        saida: String,
    }

    impl ExecutorDoShell for &ExecutorFalso {
        fn executar(&self, comando: &ComandoDoShell) -> Result<Resposta, ErroSistema> {
            self.pedidos.lock().unwrap().push(comando.clone());
            Ok(Resposta {
                sucesso: self.erro.is_none(),
                saida: self.saida.clone(),
                erro: self.erro.clone().unwrap_or_default(),
            })
        }
    }

    struct SemLeitor;

    impl LeitorDeFilas for SemLeitor {
        fn ler(&self) -> Result<Vec<FilaDoSistema>, ErroSistema> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn valor_hostil_vai_so_em_variavel_e_nunca_no_script() {
        let hostil = "x\"; Remove-Printer -Name *; $(calc) `n";
        let executor = ExecutorFalso::default();
        let shell = PowerShell::novo(&executor, SemLeitor);
        shell.criar_porta(hostil).unwrap();
        shell.criar_fila(hostil, hostil, hostil).unwrap();
        shell.remover_fila(hostil).unwrap();
        shell.remover_porta(hostil).unwrap();
        let _ = shell.porta_existe(hostil);
        let pedidos = executor.pedidos.lock().unwrap();
        assert_eq!(pedidos.len(), 5);
        for pedido in pedidos.iter() {
            assert!(!pedido.script.contains("calc"), "{}", pedido.script);
            assert!(pedido.variaveis.iter().all(|(_, valor)| valor == hostil));
            assert!(pedido.prazo <= PRAZO);
        }
    }

    #[test]
    fn falta_de_permissao_e_reconhecida_em_qualquer_idioma() {
        for linha in [
            "PLOTSIM_ERRO|PermissionDenied|0x80131500|Acesso negado",
            "PLOTSIM_ERRO|NotSpecified|0x80070005|Access is denied",
            "aviso solto\r\nPLOTSIM_ERRO|PermissionDenied|0x80041003|Zugriff verweigert\r\n",
        ] {
            assert_eq!(traduzir_erro(linha), ErroSistema::PrecisaAdministrador);
        }
    }

    #[test]
    fn outra_recusa_guarda_codigo_e_corta_o_texto() {
        let longa = format!(
            "PLOTSIM_ERRO|InvalidArgument|0x80041002|{}",
            "x".repeat(5_000)
        );
        match traduzir_erro(&longa) {
            ErroSistema::Recusado { codigo, mensagem } => {
                assert_eq!(codigo, "InvalidArgument 0x80041002");
                assert_eq!(mensagem.chars().count(), LIMITE_MENSAGEM);
            }
            outro => panic!("{outro:?}"),
        }
        assert!(matches!(
            traduzir_erro("sem marca"),
            ErroSistema::Processo(_)
        ));
        assert!(matches!(
            traduzir_erro("PLOTSIM_ERRO|"),
            ErroSistema::Recusado { .. }
        ));
    }

    #[test]
    fn resposta_estranha_sobre_a_porta_vira_erro() {
        let executor = ExecutorFalso {
            saida: "talvez".to_string(),
            ..Default::default()
        };
        let shell = PowerShell::novo(&executor, SemLeitor);
        assert!(matches!(
            shell.porta_existe("C:\\x"),
            Err(ErroSistema::Processo(_))
        ));
    }

    #[test]
    fn codificacao_do_script_segue_a_base64_em_utf16() {
        assert_eq!(script_codificado("a"), "YQA=");
        assert_eq!(script_codificado("ab"), "YQBiAA==");
        assert_eq!(script_codificado("abc"), "YQBiAGMA");
        assert_eq!(script_codificado(""), "");
    }
}
