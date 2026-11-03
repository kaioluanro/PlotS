//! `plotsim suite`: cria filas virtuais, imprime pelo PlotS em modo de teste e confere o maço.

use plotsim_suite::matriz::{matriz_completa, matriz_rapida};
use std::path::PathBuf;
use std::process::ExitCode;

const USO: &str = "Uso:
  plotsim suite --pasta PASTA [--rapida] [--plots ARQUIVO] [--portas PASTA] [--filtro TEXTO] [--listar]
Cria filas 'PlotSim — Suite ...', roda a matriz com o 'plots imprimir --teste' de verdade e grava relatorio.html em PASTA, fora do repositório. As filas são removidas ao fim, mesmo com falha.
--rapida roda um recorte de cada dimensão. Sem ela, roda a matriz inteira, que leva horas.
--plots indica o executável do PlotS; sem ele, usa o plots-cli ao lado deste programa.
--portas indica onde as filas gravam; sem ela, usa %ProgramData%/PlotSim.
--filtro só roda os cenários com TEXTO no nome.
--listar só mostra os cenários e não mexe em nada.
Precisa de PLOTS_PDFIUM_DIR ou da pdfium.dll ao lado do executável.
";

#[derive(Debug, Default, PartialEq)]
struct Pedido {
    pasta: Option<PathBuf>,
    plots: Option<PathBuf>,
    portas: Option<PathBuf>,
    filtro: Option<String>,
    rapida: bool,
    listar: bool,
}

fn analisar(args: &[String]) -> Result<Pedido, String> {
    let mut iterador = args.iter();
    if iterador.next().map(String::as_str) != Some("suite") {
        return Err("use 'plotsim suite'".to_string());
    }
    let mut pedido = Pedido::default();
    while let Some(chave) = iterador.next() {
        let mut valor = || {
            iterador
                .next()
                .cloned()
                .ok_or_else(|| format!("faltou valor para {chave}"))
        };
        match chave.as_str() {
            "--rapida" if !pedido.rapida => pedido.rapida = true,
            "--listar" if !pedido.listar => pedido.listar = true,
            "--pasta" if pedido.pasta.is_none() => pedido.pasta = Some(PathBuf::from(valor()?)),
            "--plots" if pedido.plots.is_none() => pedido.plots = Some(PathBuf::from(valor()?)),
            "--portas" if pedido.portas.is_none() => pedido.portas = Some(PathBuf::from(valor()?)),
            "--filtro" if pedido.filtro.is_none() => pedido.filtro = Some(valor()?),
            _ => return Err(format!("opção desconhecida ou repetida: {chave}")),
        }
    }
    Ok(pedido)
}

fn cenarios_do_pedido(pedido: &Pedido) -> Vec<plotsim_suite::matriz::Cenario> {
    let todos = if pedido.rapida {
        matriz_rapida()
    } else {
        matriz_completa()
    };
    match &pedido.filtro {
        Some(texto) => todos
            .into_iter()
            .filter(|cenario| cenario.nome().contains(texto.as_str()))
            .collect(),
        None => todos,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pedido = match analisar(&args) {
        Ok(pedido) => pedido,
        Err(erro) => {
            eprintln!("Erro: {erro}\n\n{USO}");
            return ExitCode::from(2);
        }
    };
    if pedido.listar {
        let cenarios = cenarios_do_pedido(&pedido);
        for cenario in &cenarios {
            println!("{}", cenario.nome());
        }
        println!("{} cenários", cenarios.len());
        return ExitCode::SUCCESS;
    }
    rodar(&pedido)
}

#[cfg(not(windows))]
fn rodar(_pedido: &Pedido) -> ExitCode {
    eprintln!("A suíte só roda no Windows.");
    ExitCode::from(2)
}

#[cfg(windows)]
fn rodar(pedido: &Pedido) -> ExitCode {
    no_windows::rodar(pedido)
}

#[cfg(windows)]
mod no_windows {
    use super::*;
    use plotsim_suite::execucao::{filas_da_suite_no_sistema, Ambiente, ErroDaSuite};
    use plotsim_suite::relatorio::{contar, gerar_html, Estado, ResultadoDoCenario};
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows::core::BOOL;
    use windows::Win32::System::Console::SetConsoleCtrlHandler;

    static PARAR: AtomicBool = AtomicBool::new(false);

    unsafe extern "system" fn ao_interromper(_tipo: u32) -> BOOL {
        PARAR.store(true, Ordering::Relaxed);
        BOOL(1)
    }

    /// A pasta do relatório fica fora de qualquer repositório.
    fn dentro_de_repositorio(pasta: &Path) -> bool {
        pasta.ancestors().any(|acima| acima.join(".git").exists())
    }

    fn gravar(pasta: &Path, resultados: &[ResultadoDoCenario], restou: &str, avisos: &[String]) {
        let html = gerar_html("Suíte do PlotSim", resultados, restou, avisos);
        if let Err(erro) = std::fs::write(pasta.join("relatorio.html"), html) {
            eprintln!("não deu para gravar o relatório: {erro}");
        }
    }

    fn pasta_das_portas(pedido: &Pedido) -> Result<PathBuf, String> {
        if let Some(pasta) = &pedido.portas {
            return Ok(pasta.clone());
        }
        std::env::var_os("ProgramData")
            .map(|pasta| PathBuf::from(pasta).join("PlotSim"))
            .ok_or_else(|| "a variável ProgramData não está definida".to_string())
    }

    fn programa_plots(pedido: &Pedido) -> Result<PathBuf, String> {
        if let Some(plots) = &pedido.plots {
            return Ok(plots.clone());
        }
        std::env::current_exe()
            .map_err(|erro| erro.to_string())?
            .parent()
            .map(|pasta| pasta.join("plots-cli.exe"))
            .ok_or_else(|| "não deu para achar a pasta do programa".to_string())
    }

    pub fn rodar(pedido: &Pedido) -> ExitCode {
        let Some(pasta) = pedido.pasta.clone() else {
            eprintln!("Erro: informe --pasta\n\n{USO}");
            return ExitCode::from(2);
        };
        let absoluta = std::path::absolute(&pasta).unwrap_or_else(|_| pasta.clone());
        if dentro_de_repositorio(&absoluta) {
            eprintln!(
                "Erro: {} está dentro de um repositório; o relatório fica fora dele",
                absoluta.display()
            );
            return ExitCode::from(2);
        }
        if let Err(erro) = std::fs::create_dir_all(&absoluta) {
            eprintln!("Erro: não deu para criar {}: {erro}", absoluta.display());
            return ExitCode::from(2);
        }
        let (plots, portas) = match (programa_plots(pedido), pasta_das_portas(pedido)) {
            (Ok(plots), Ok(portas)) => (plots, portas),
            (Err(erro), _) | (_, Err(erro)) => {
                eprintln!("Erro: {erro}");
                return ExitCode::from(2);
            }
        };
        if !plots.is_file() {
            eprintln!("Erro: {} não existe; indique com --plots", plots.display());
            return ExitCode::from(2);
        }
        // SAFETY: a função registrada só mexe em um valor atômico e vive o programa inteiro.
        let _ = unsafe { SetConsoleCtrlHandler(Some(ao_interromper), true) };
        let cenarios = cenarios_do_pedido(pedido);
        let trabalho = absoluta.join("trabalho");
        let mut resultados = Vec::with_capacity(cenarios.len());
        let mut parada = None;
        {
            let mut ambiente = match Ambiente::novo(plots, trabalho, portas, &PARAR) {
                Ok(ambiente) => ambiente,
                Err(erro) => {
                    eprintln!("Erro: {erro}");
                    return ExitCode::FAILURE;
                }
            };
            for (indice, cenario) in cenarios.iter().enumerate() {
                println!("[{}/{}] {}", indice + 1, cenarios.len(), cenario.nome());
                match ambiente.rodar(cenario) {
                    Ok(resultado) => {
                        println!(
                            "  {} em {} s: {}",
                            match resultado.estado {
                                Estado::Passou => "passou",
                                Estado::Falhou => "FALHOU",
                                Estado::SemCobertura => "sem cobertura",
                            },
                            resultado.duracao.as_secs(),
                            resultado.motivo
                        );
                        resultados.push(resultado);
                        gravar(&absoluta, &resultados, "Filas ainda em uso.", &[]);
                    }
                    Err(erro @ (ErroDaSuite::Negado(_) | ErroDaSuite::Cancelada)) => {
                        parada = Some(erro.to_string());
                        break;
                    }
                    Err(erro) => {
                        parada = Some(erro.to_string());
                        break;
                    }
                }
            }
            // O ambiente sai de cena aqui: canal fechado e filas removidas, mesmo com falha.
        }
        let restou = match filas_da_suite_no_sistema() {
            Ok(filas) if filas.is_empty() => "Nenhuma fila da suíte restou no Windows.".to_string(),
            Ok(filas) => format!("Filas que restaram no Windows: {}.", filas.join(", ")),
            Err(erro) => format!("Não deu para conferir as filas no fim: {erro}."),
        };
        let avisos: Vec<String> = parada
            .iter()
            .map(|motivo| format!("A suíte parou antes do fim: {motivo}."))
            .collect();
        gravar(&absoluta, &resultados, &restou, &avisos);
        let contagem = contar(&resultados);
        println!(
            "{} de {} passaram, {} sem cobertura, {} falharam. {restou}",
            contagem.passaram,
            resultados.len(),
            contagem.sem_cobertura,
            contagem.falharam
        );
        if let Some(motivo) = &parada {
            eprintln!("Erro: {motivo}");
        }
        println!("Relatório: {}", absoluta.join("relatorio.html").display());
        if parada.is_none() && contagem.falharam == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn args(texto: &[&str]) -> Vec<String> {
        texto.iter().map(|t| (*t).to_string()).collect()
    }

    #[test]
    fn analisa_as_opcoes() {
        let pedido = analisar(&args(&[
            "suite", "--rapida", "--pasta", "x", "--plots", "y", "--filtro", "z",
        ]))
        .unwrap();
        assert!(pedido.rapida);
        assert_eq!(pedido.pasta, Some(PathBuf::from("x")));
        assert_eq!(pedido.plots, Some(PathBuf::from("y")));
        assert_eq!(pedido.filtro.as_deref(), Some("z"));
    }

    #[test]
    fn recusa_o_que_nao_conhece() {
        for ruim in [
            &["outra"][..],
            &[][..],
            &["suite", "--xis"][..],
            &["suite", "--pasta"][..],
            &["suite", "--rapida", "--rapida"][..],
            &["suite", "--pasta", "a", "--pasta", "b"][..],
        ] {
            assert!(analisar(&args(ruim)).is_err(), "{ruim:?}");
        }
    }

    #[test]
    fn o_filtro_escolhe_pelo_nome() {
        let pedido = analisar(&args(&["suite", "--rapida", "--filtro", "espelho"])).unwrap();
        let cenarios = cenarios_do_pedido(&pedido);
        assert!(!cenarios.is_empty());
        assert!(cenarios.iter().all(|c| c.nome().contains("espelho")));
    }
}
