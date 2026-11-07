//! Separação CMYK das páginas por um `mutool.exe` externo e leitura da cobertura de cada PAM.
//!
//! O conversor é um processo separado: o PlotS nunca o liga ao binário. A pasta de trabalho é
//! exclusiva de cada lote e some ao fim, inclusive quando o processo falha, estoura o tempo ou é
//! cancelado. As páginas são números do documento original: o `mutool` grava `p-0001.pam` para a
//! página 1, e assim por diante.
//!
//! O binário é aceito como está. A licença do MuPDF permite versões alteradas, então uma
//! conferência de hash, quando existir, é apenas informativa e nunca recusa o conversor.

use plots_info::{cobertura_de_pam, Cobertura};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Tempo máximo de um lote, quando quem chama não indica outro.
pub const LIMITE_PADRAO: Duration = Duration::from_secs(120);

/// Maior PAM lido inteiro. Um A2 a 300 dpi em CMYK ocupa cerca de 278 MiB.
pub const LIMITE_PAM_BYTES: u64 = 512 * 1024 * 1024;

/// Maior quantidade de páginas pedida em um lote.
pub const LIMITE_PAGINAS: usize = 10_000;

/// Maior texto de intervalo aceito na linha de comando do Windows.
const LIMITE_ARGUMENTO_PAGINAS: usize = 28_000;

/// Nome do arquivo que o `mutool` grava para cada página.
const PADRAO_DE_SAIDA: &str = "p-%04d.pam";

/// Enquanto o processo roda, o estado é conferido neste intervalo.
const INTERVALO_DE_ESPERA: Duration = Duration::from_millis(25);

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

static CONTADOR_DE_PASTAS: AtomicU64 = AtomicU64::new(0);

/// Cenário de um lote em CMYK que não chegou ao fim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroMutool {
    PdfInvalido,
    DpiInvalido {
        dpi: u32,
    },
    SemPaginas,
    PaginasDemais {
        paginas: usize,
        limite: usize,
    },
    PaginaInvalida {
        pagina: u32,
    },
    LimiteInvalido,
    Temporario {
        detalhe: String,
    },
    Iniciar {
        detalhe: String,
    },
    Processo {
        detalhe: String,
    },
    Falhou {
        codigo: Option<i32>,
    },
    TempoEsgotado {
        limite: Duration,
    },
    Cancelado,
    PamFaltando {
        pagina: u32,
    },
    PamGrande {
        pagina: u32,
        bytes: u64,
        limite: u64,
    },
    PamInvalido {
        pagina: u32,
        motivo: String,
    },
}

impl std::fmt::Display for ErroMutool {
    fn fmt(&self, saida: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PdfInvalido => write!(saida, "o arquivo PDF não pôde ser lido"),
            Self::DpiInvalido { dpi } => {
                write!(saida, "a resolução {dpi} dpi não é aceita para a análise")
            }
            Self::SemPaginas => write!(saida, "nenhuma página foi escolhida para a análise"),
            Self::PaginasDemais { paginas, limite } => write!(
                saida,
                "a análise pediu {paginas} páginas e o limite é {limite}"
            ),
            Self::PaginaInvalida { pagina } => {
                write!(saida, "a página {pagina} não é um número válido de página")
            }
            Self::LimiteInvalido => write!(saida, "o tempo limite da análise precisa ser positivo"),
            Self::Temporario { detalhe } => {
                write!(
                    saida,
                    "não foi possível preparar a pasta da análise: {detalhe}"
                )
            }
            Self::Iniciar { detalhe } => {
                write!(
                    saida,
                    "não foi possível iniciar o conversor mutool: {detalhe}"
                )
            }
            Self::Processo { detalhe } => {
                write!(
                    saida,
                    "o conversor mutool não pôde ser acompanhado: {detalhe}"
                )
            }
            Self::Falhou { codigo } => match codigo {
                Some(codigo) => write!(saida, "o conversor mutool terminou com o código {codigo}"),
                None => write!(saida, "o conversor mutool terminou sem informar um código"),
            },
            Self::TempoEsgotado { limite } => write!(
                saida,
                "a análise passou do tempo limite de {} segundos",
                limite.as_secs()
            ),
            Self::Cancelado => write!(saida, "a análise foi cancelada"),
            Self::PamFaltando { pagina } => {
                write!(saida, "o conversor não gravou a página {pagina}")
            }
            Self::PamGrande {
                pagina,
                bytes,
                limite,
            } => write!(
                saida,
                "a página {pagina} gerou {bytes} bytes e passa do limite de {limite}"
            ),
            Self::PamInvalido { pagina, motivo } => {
                write!(saida, "a página {pagina} gerou um PAM inválido: {motivo}")
            }
        }
    }
}

impl std::error::Error for ErroMutool {}

/// Cobertura CMYK de uma página, lida do PAM que o conversor gravou.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaginaCmyk {
    pub pagina: u32,
    pub cobertura: Cobertura,
}

/// Caminho do conversor: `PLOTS_MUTOOL`, depois `mutool\mutool.exe` e `mutool.exe` ao lado do
/// executável.
pub fn mutool_do_sistema() -> Option<PathBuf> {
    let executavel = std::env::current_exe().ok();
    let base = executavel
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    let ambiente = std::env::var_os("PLOTS_MUTOOL");
    mutool_procurado(base.as_deref(), ambiente.as_deref().map(Path::new))
}

/// Procura o conversor nas posições combinadas. A pasta base e o caminho do ambiente entram como
/// parâmetros para a procura ser testável sem mexer no ambiente do processo.
pub fn mutool_procurado(base: Option<&Path>, ambiente: Option<&Path>) -> Option<PathBuf> {
    if let Some(caminho) = ambiente.filter(|caminho| caminho.is_file()) {
        return Some(caminho.to_path_buf());
    }
    let base = base?;
    let na_subpasta = base.join("mutool").join("mutool.exe");
    if na_subpasta.is_file() {
        return Some(na_subpasta);
    }
    let ao_lado = base.join("mutool.exe");
    if ao_lado.is_file() {
        return Some(ao_lado);
    }
    None
}

/// Separa as páginas indicadas em CMYK e devolve a cobertura de cada uma, em ordem crescente.
///
/// O PDF precisa ser o conteúdo já resolvido para leitura (o próprio arquivo ou o derivado de uma
/// imagem). Quem chama passa o conversor já resolvido; quando ele falta, a análise usa o modo
/// simples.
pub fn desenhar_cmyk_com(
    mutool: &Path,
    pdf: &Path,
    paginas: &[u32],
    dpi: u32,
    cinzas_so_preto: bool,
    limite: Duration,
    cancelado: &AtomicBool,
) -> Result<Vec<PaginaCmyk>, ErroMutool> {
    desenhar_cmyk_em(None, mutool, pdf, paginas, dpi, cinzas_so_preto, limite, cancelado)
}

fn desenhar_cmyk_em(
    base: Option<&Path>,
    mutool: &Path,
    pdf: &Path,
    paginas: &[u32],
    dpi: u32,
    cinzas_so_preto: bool,
    limite: Duration,
    cancelado: &AtomicBool,
) -> Result<Vec<PaginaCmyk>, ErroMutool> {
    let ordenadas = validar(pdf, paginas, dpi)?;
    if limite.is_zero() {
        return Err(ErroMutool::LimiteInvalido);
    }
    if cancelado.load(Ordering::SeqCst) {
        return Err(ErroMutool::Cancelado);
    }
    let intervalo = intervalo_de_paginas(&ordenadas)?;
    let pasta = PastaTemporaria::nova(base)?;
    let mut argumentos: Vec<OsString> = Vec::with_capacity(10);
    for parte in ["draw", "-q", "-c", "cmyk"] {
        argumentos.push(OsString::from(parte));
    }
    argumentos.push(OsString::from("-r"));
    argumentos.push(OsString::from(dpi.to_string()));
    argumentos.push(OsString::from("-o"));
    argumentos.push(pasta.caminho.join(PADRAO_DE_SAIDA).into_os_string());
    argumentos.push(pdf.as_os_str().to_owned());
    argumentos.push(OsString::from(intervalo));
    rodar_processo(mutool.as_os_str(), &argumentos, limite, cancelado)?;
    ler_paginas(&pasta.caminho, &ordenadas, cinzas_so_preto)
}

fn validar(pdf: &Path, paginas: &[u32], dpi: u32) -> Result<Vec<u32>, ErroMutool> {
    if !(1..=1200).contains(&dpi) {
        return Err(ErroMutool::DpiInvalido { dpi });
    }
    if !fs::metadata(pdf).is_ok_and(|metadados| metadados.is_file()) {
        return Err(ErroMutool::PdfInvalido);
    }
    if paginas.is_empty() {
        return Err(ErroMutool::SemPaginas);
    }
    if paginas.len() > LIMITE_PAGINAS {
        return Err(ErroMutool::PaginasDemais {
            paginas: paginas.len(),
            limite: LIMITE_PAGINAS,
        });
    }
    if paginas.contains(&0) {
        return Err(ErroMutool::PaginaInvalida { pagina: 0 });
    }
    let mut ordenadas = paginas.to_vec();
    ordenadas.sort_unstable();
    ordenadas.dedup();
    Ok(ordenadas)
}

/// Intervalo no formato que o `mutool` entende: páginas seguidas viram uma faixa e o resto entra
/// separado por vírgulas.
fn intervalo_de_paginas(paginas: &[u32]) -> Result<String, ErroMutool> {
    let Some(&primeira) = paginas.first() else {
        return Err(ErroMutool::SemPaginas);
    };
    let mut partes = Vec::new();
    let mut inicio = primeira;
    let mut anterior = primeira;
    for &pagina in paginas.iter().skip(1) {
        if anterior.checked_add(1) == Some(pagina) {
            anterior = pagina;
            continue;
        }
        partes.push(parte(inicio, anterior));
        inicio = pagina;
        anterior = pagina;
    }
    partes.push(parte(inicio, anterior));
    let intervalo = partes.join(",");
    if intervalo.len() > LIMITE_ARGUMENTO_PAGINAS {
        return Err(ErroMutool::PaginasDemais {
            paginas: paginas.len(),
            limite: LIMITE_PAGINAS,
        });
    }
    Ok(intervalo)
}

fn parte(inicio: u32, fim: u32) -> String {
    if inicio == fim {
        inicio.to_string()
    } else {
        format!("{inicio}-{fim}")
    }
}

fn rodar_processo(
    programa: &OsStr,
    argumentos: &[OsString],
    limite: Duration,
    cancelado: &AtomicBool,
) -> Result<(), ErroMutool> {
    if limite.is_zero() {
        return Err(ErroMutool::LimiteInvalido);
    }
    if cancelado.load(Ordering::SeqCst) {
        return Err(ErroMutool::Cancelado);
    }
    let mut comando = Command::new(programa);
    comando
        .args(argumentos)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        comando.creation_flags(CREATE_NO_WINDOW);
    }
    let mut filho = comando.spawn().map_err(|falha| ErroMutool::Iniciar {
        detalhe: falha.to_string(),
    })?;
    let inicio = Instant::now();
    let situacao = loop {
        if cancelado.load(Ordering::SeqCst) {
            encerrar(&mut filho);
            return Err(ErroMutool::Cancelado);
        }
        if inicio.elapsed() >= limite {
            encerrar(&mut filho);
            return Err(ErroMutool::TempoEsgotado { limite });
        }
        match filho.try_wait() {
            Ok(Some(situacao)) => break situacao,
            Ok(None) => std::thread::sleep(INTERVALO_DE_ESPERA),
            Err(falha) => {
                encerrar(&mut filho);
                return Err(ErroMutool::Processo {
                    detalhe: falha.to_string(),
                });
            }
        }
    };
    if situacao.success() {
        Ok(())
    } else {
        Err(ErroMutool::Falhou {
            codigo: situacao.code(),
        })
    }
}

/// Mata apenas o processo que este lote abriu, pelo identificador que o sistema deu a ele.
fn encerrar(filho: &mut Child) {
    let _ = filho.kill();
    let _ = filho.wait();
}

/// Pasta de trabalho exclusiva do lote; o `Drop` apaga tudo, inclusive os PAMs que sobraram.
struct PastaTemporaria {
    caminho: PathBuf,
}

impl PastaTemporaria {
    fn nova(base: Option<&Path>) -> Result<Self, ErroMutool> {
        let base = base.map_or_else(std::env::temp_dir, Path::to_path_buf);
        for _ in 0..16 {
            let sequencia = CONTADOR_DE_PASTAS.fetch_add(1, Ordering::Relaxed);
            let caminho = base.join(format!("plots-mutool-{}-{sequencia}", std::process::id()));
            match fs::create_dir(&caminho) {
                Ok(()) => return Ok(Self { caminho }),
                Err(falha) if falha.kind() == ErrorKind::AlreadyExists => continue,
                Err(falha) => {
                    return Err(ErroMutool::Temporario {
                        detalhe: falha.to_string(),
                    })
                }
            }
        }
        Err(ErroMutool::Temporario {
            detalhe: "não há nome livre para a pasta da análise".to_owned(),
        })
    }
}

impl Drop for PastaTemporaria {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.caminho);
    }
}

/// Nome do PAM de uma página, o mesmo que o `%04d` do conversor produz.
pub fn nome_do_pam(pagina: u32) -> String {
    format!("p-{pagina:04}.pam")
}

fn ler_paginas(
    pasta: &Path,
    paginas: &[u32],
    cinzas_so_preto: bool,
) -> Result<Vec<PaginaCmyk>, ErroMutool> {
    let mut resultado = Vec::with_capacity(paginas.len());
    for &pagina in paginas {
        let caminho = pasta.join(nome_do_pam(pagina));
        let bytes = ler_pam(&caminho, pagina)?;
        let cobertura = cobertura_de_pam(&bytes, cinzas_so_preto).map_err(|falha| ErroMutool::PamInvalido {
            pagina,
            motivo: falha.to_string(),
        })?;
        let _ = fs::remove_file(&caminho);
        resultado.push(PaginaCmyk { pagina, cobertura });
    }
    Ok(resultado)
}

fn ler_pam(caminho: &Path, pagina: u32) -> Result<Vec<u8>, ErroMutool> {
    let metadados = match fs::metadata(caminho) {
        Ok(metadados) => metadados,
        Err(falha) if falha.kind() == ErrorKind::NotFound => {
            return Err(ErroMutool::PamFaltando { pagina })
        }
        Err(falha) => {
            return Err(ErroMutool::PamInvalido {
                pagina,
                motivo: falha.to_string(),
            })
        }
    };
    if !metadados.is_file() {
        return Err(ErroMutool::PamFaltando { pagina });
    }
    if metadados.len() > LIMITE_PAM_BYTES {
        return Err(ErroMutool::PamGrande {
            pagina,
            bytes: metadados.len(),
            limite: LIMITE_PAM_BYTES,
        });
    }
    fs::read(caminho).map_err(|falha| ErroMutool::PamInvalido {
        pagina,
        motivo: falha.to_string(),
    })
}

#[cfg(test)]
pub(crate) mod testes {
    use super::*;
    use std::sync::atomic::AtomicU64;

    static CONTADOR: AtomicU64 = AtomicU64::new(0);

    struct Pasta(PathBuf);

    impl Pasta {
        fn nova() -> Self {
            let sequencia = CONTADOR.fetch_add(1, Ordering::Relaxed);
            let caminho = std::env::temp_dir().join(format!(
                "plots-mutool-teste-{}-{sequencia}",
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

        fn criar(&self, nome: &str, bytes: &[u8]) -> PathBuf {
            let caminho = self.0.join(nome);
            if let Some(pasta) = caminho.parent() {
                fs::create_dir_all(pasta).expect("criar pasta do arquivo");
            }
            fs::write(&caminho, bytes).expect("criar arquivo do teste");
            caminho
        }

        fn vazia(&self) -> bool {
            fs::read_dir(&self.0).map_or(true, |mut entradas| entradas.next().is_none())
        }
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// PDF mínimo com uma página por valor de ciano, preenchidas de ponta a ponta em DeviceCMYK.
    pub(crate) fn pdf_cmyk(cianos: &[f64]) -> Vec<u8> {
        let mut objetos: Vec<Vec<u8>> = Vec::new();
        objetos.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
        let filhos: Vec<String> = (0..cianos.len())
            .map(|n| format!("{} 0 R", 3 + n))
            .collect();
        objetos.push(
            format!(
                "<< /Type /Pages /Count {} /Kids [{}] >>",
                cianos.len(),
                filhos.join(" ")
            )
            .into_bytes(),
        );
        for indice in 0..cianos.len() {
            objetos.push(
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << >> \
                     /Contents {} 0 R >>",
                    3 + cianos.len() + indice
                )
                .into_bytes(),
            );
        }
        for ciano in cianos {
            let conteudo = format!("q {ciano} 0 0 0 k 0 0 595 842 re f Q").into_bytes();
            let mut objeto = format!("<< /Length {} >>\nstream\n", conteudo.len()).into_bytes();
            objeto.extend_from_slice(&conteudo);
            objeto.extend_from_slice(b"\nendstream");
            objetos.push(objeto);
        }
        montar_pdf(&objetos)
    }

    fn montar_pdf(objetos: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = b"%PDF-1.5\n".to_vec();
        let mut posicoes = Vec::with_capacity(objetos.len());
        for (indice, objeto) in objetos.iter().enumerate() {
            posicoes.push(bytes.len());
            bytes.extend_from_slice(format!("{} 0 obj\n", indice + 1).as_bytes());
            bytes.extend_from_slice(objeto);
            bytes.extend_from_slice(b"\nendobj\n");
        }
        let tabela = bytes.len();
        let total = objetos.len() + 1;
        bytes.extend_from_slice(format!("xref\n0 {total}\n0000000000 65535 f \n").as_bytes());
        for posicao in posicoes {
            bytes.extend_from_slice(format!("{posicao:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size {total} /Root 1 0 R >>\nstartxref\n{tabela}\n%%EOF\n")
                .as_bytes(),
        );
        bytes
    }

    #[test]
    fn procura_o_mutool_no_ambiente_antes_das_pastas_ao_lado() {
        let pasta = Pasta::nova();
        let ao_lado = pasta.criar("mutool.exe", b"plano");
        let na_subpasta = pasta.criar("mutool/mutool.exe", b"subpasta");
        let do_ambiente = pasta.criar("outro/mutool.exe", b"ambiente");

        assert_eq!(
            mutool_procurado(Some(&pasta.0), Some(&do_ambiente)),
            Some(do_ambiente.clone())
        );
        assert_eq!(
            mutool_procurado(Some(&pasta.0), None),
            Some(na_subpasta.clone())
        );
        fs::remove_file(&na_subpasta).expect("remover a cópia da subpasta");
        assert_eq!(mutool_procurado(Some(&pasta.0), None), Some(ao_lado));
    }

    #[test]
    fn sem_mutool_configurado_nao_ha_caminho() {
        let pasta = Pasta::nova();
        assert_eq!(mutool_procurado(Some(&pasta.0), None), None);
        assert_eq!(mutool_procurado(None, None), None);
        let sumido = pasta.0.join("nao-existe/mutool.exe");
        assert_eq!(mutool_procurado(Some(&pasta.0), Some(&sumido)), None);
    }

    #[test]
    fn intervalo_compacta_paginas_seguidas_e_mantem_as_soltas() {
        assert_eq!(intervalo_de_paginas(&[2, 3]), Ok("2-3".to_owned()));
        assert_eq!(
            intervalo_de_paginas(&[2, 5, 7, 9]),
            Ok("2,5,7,9".to_owned())
        );
        assert_eq!(
            intervalo_de_paginas(&[1, 2, 3, 5, 6, 9]),
            Ok("1-3,5-6,9".to_owned())
        );
        assert_eq!(intervalo_de_paginas(&[]), Err(ErroMutool::SemPaginas));
    }

    #[test]
    fn paginas_e_resolucao_hostis_sao_recusadas_antes_de_abrir_o_conversor() {
        let pasta = Pasta::nova();
        let pdf = pasta.gravar("tres.pdf", &pdf_cmyk(&[0.1, 0.2, 0.3]));

        assert_eq!(validar(&pdf, &[], 72), Err(ErroMutool::SemPaginas));
        assert_eq!(
            validar(&pdf, &[0], 72),
            Err(ErroMutool::PaginaInvalida { pagina: 0 })
        );
        assert_eq!(
            validar(&pdf, &[1], 0),
            Err(ErroMutool::DpiInvalido { dpi: 0 })
        );
        assert_eq!(
            validar(&pdf, &[1], 1201),
            Err(ErroMutool::DpiInvalido { dpi: 1201 })
        );
        assert_eq!(
            validar(&pasta.0.join("sumiu.pdf"), &[1], 72),
            Err(ErroMutool::PdfInvalido)
        );
        assert_eq!(validar(&pasta.0, &[1], 72), Err(ErroMutool::PdfInvalido));
    }

    #[test]
    fn pasta_temporaria_e_apagada_mesmo_quando_o_lote_falha() {
        let pasta = Pasta::nova();
        let resultado: Result<(), ErroMutool> = (|| {
            let _temporaria = PastaTemporaria::nova(Some(&pasta.0))?;
            Err(ErroMutool::Cancelado)
        })();
        assert_eq!(resultado, Err(ErroMutool::Cancelado));
        assert!(pasta.vazia(), "a pasta temporária deveria ter sido apagada");
    }

    #[test]
    fn nomes_de_pam_seguem_o_numero_original_da_pagina() {
        assert_eq!(nome_do_pam(1), "p-0001.pam");
        assert_eq!(nome_do_pam(2), "p-0002.pam");
        assert_eq!(nome_do_pam(12345), "p-12345.pam");
    }

    #[cfg(windows)]
    #[test]
    fn processo_que_estoura_o_tempo_e_encerrado() {
        let limite = Duration::from_millis(300);
        let inicio = Instant::now();
        let resultado = rodar_processo(
            OsStr::new("ping.exe"),
            &[
                OsString::from("-n"),
                OsString::from("60"),
                OsString::from("127.0.0.1"),
            ],
            limite,
            &AtomicBool::new(false),
        );
        assert_eq!(resultado, Err(ErroMutool::TempoEsgotado { limite }));
        assert!(
            inicio.elapsed() < Duration::from_secs(10),
            "o processo demorou demais para ser encerrado"
        );
    }

    #[cfg(windows)]
    #[test]
    fn processo_cancelado_e_encerrado_pelo_pid() {
        use std::sync::Arc;

        let cancelado = Arc::new(AtomicBool::new(false));
        let do_lote = Arc::clone(&cancelado);
        let inicio = Instant::now();
        let lote = std::thread::spawn(move || {
            rodar_processo(
                OsStr::new("ping.exe"),
                &[
                    OsString::from("-n"),
                    OsString::from("60"),
                    OsString::from("127.0.0.1"),
                ],
                Duration::from_secs(120),
                &do_lote,
            )
        });
        std::thread::sleep(Duration::from_millis(200));
        cancelado.store(true, Ordering::SeqCst);
        let resultado = lote.join().expect("a thread do lote terminou");
        assert_eq!(resultado, Err(ErroMutool::Cancelado));
        assert!(
            inicio.elapsed() < Duration::from_secs(10),
            "o cancelamento demorou demais para encerrar o processo"
        );
    }

    #[test]
    #[ignore = "depende do mutool real em PLOTS_MUTOOL"]
    fn mutool_real_le_o_ciano_de_cada_pagina() {
        let Some(mutool) = mutool_do_sistema() else {
            eprintln!("PLOTS_MUTOOL não está configurado; teste do conversor real ignorado");
            return;
        };
        let pasta = Pasta::nova();
        // O nome com espaço e acento também prova que o caminho chega ao processo sem quebrar.
        let pdf = pasta.gravar("três páginas ação.pdf", &pdf_cmyk(&[0.1, 0.2, 0.3]));
        // O mutool grava a página 2 e a 3 pelos números originais.
        let saida = desenhar_cmyk_em(
            Some(&pasta.0),
            &mutool,
            &pdf,
            &[2, 3],
            72,
            false,
            Duration::from_secs(120),
            &AtomicBool::new(false),
        )
        .expect("as páginas 2 e 3 deveriam ser separadas em CMYK");
        assert_eq!(
            saida.iter().map(|item| item.pagina).collect::<Vec<_>>(),
            vec![2, 3]
        );

        // Uma segunda execução pede as três páginas; os PAMs existem só durante a leitura.
        let saida = desenhar_cmyk_em(
            Some(&pasta.0),
            &mutool,
            &pdf,
            &[1, 2, 3],
            72,
            false,
            Duration::from_secs(120),
            &AtomicBool::new(false),
        )
        .expect("as três páginas deveriam ser separadas em CMYK");
        assert_eq!(
            saida.iter().map(|item| item.pagina).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        let esperado = [
            (25u8, 25.0 / 255.0 * 100.0),
            (51, 51.0 / 255.0 * 100.0),
            (76, 76.0 / 255.0 * 100.0),
        ];
        for (item, (byte, percentual)) in saida.iter().zip(esperado) {
            let dentro = (item.cobertura.c - percentual).abs() < 1e-9;
            assert!(
                dentro,
                "página {}: ciano {:.4}, esperado {:.4}",
                item.pagina, item.cobertura.c, percentual
            );
            assert_eq!(item.cobertura.m, 0.0, "página {}", item.pagina);
            assert_eq!(item.cobertura.y, 0.0, "página {}", item.pagina);
            assert_eq!(item.cobertura.k, 0.0, "página {}", item.pagina);
            // A cor pedida foi 10%, 20% e 30%; o PAM de 8 bits trunca o valor.
            assert!(
                (item.cobertura.c - (byte as f64 / 255.0 * 100.0)).abs() < 1e-9,
                "página {}: byte do ciano não confere",
                item.pagina
            );
            println!("página {}: ciano {:.4}%", item.pagina, item.cobertura.c);
        }
        let restantes: Vec<String> = fs::read_dir(&pasta.0)
            .expect("ler a pasta do teste")
            .flatten()
            .map(|entrada| entrada.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            restantes,
            vec!["três páginas ação.pdf".to_owned()],
            "a pasta do teste deveria ficar só com o PDF depois da limpeza"
        );
    }
}
