use plots_core::{
    pares_com_orientacao_mista, plano_documento, plano_lotes_por_papel, OrdemPassadas,
    PerfilImpressora, LIMITE_PAGINAS,
};
use plots_print::{
    job::{EstadoJob, Job, SimBackend},
    Mecanica, Recolocacao,
};
use std::io::{BufRead, Write};

/// Texto de uso geral, mostrado por `--help` e em erros sem subcomando conhecido.
pub const USO_GERAL: &str = "Uso:\n  plots simular --paginas N --lote K --impressora traseira|gaveta [--guiado]\n  plots gerar-pdf ARQUIVO --lote K --impressora traseira|gaveta --saida DIRETORIO\nNenhum comando envia arquivos à impressora: 'simular' só simula e 'gerar-pdf' só grava arquivos.\n";

/// Texto de uso do subcomando `simular`.
pub const USO_SIMULAR: &str = "Uso:\n  plots simular --paginas N --lote K --impressora traseira|gaveta [--guiado]\nA simulação não envia nada à impressora.\n";

/// Texto de uso do subcomando `gerar-pdf`.
pub const USO_GERAR_PDF: &str = "Uso:\n  plots gerar-pdf ARQUIVO --lote K --impressora traseira|gaveta --saida DIRETORIO\nOs PDFs gerados não são enviados à impressora.\n";

const MSG_COMANDO_DESCONHECIDO: &str =
    "comando ausente ou desconhecido; use 'simular' ou 'gerar-pdf'";
const MSG_SIMULAR_PDF: &str = "'simular' não lê PDF: informe --paginas N, ou use 'gerar-pdf' para gerar as passadas de um arquivo";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImpressoraVirtual {
    Traseira,
    Gaveta,
}

impl ImpressoraVirtual {
    fn analisar(valor: &str) -> Result<Self, String> {
        match valor {
            "traseira" => Ok(Self::Traseira),
            "gaveta" => Ok(Self::Gaveta),
            _ => Err(format!(
                "impressora virtual desconhecida: {valor:?}; use traseira ou gaveta"
            )),
        }
    }

    fn perfil(self) -> PerfilImpressora {
        PerfilImpressora {
            passada1_inversa: self == Self::Traseira,
            passada2_inversa: self == Self::Traseira,
            ..PerfilImpressora::default()
        }
    }

    fn mecanica(self) -> Mecanica {
        match self {
            Self::Traseira => Mecanica::traseira(),
            Self::Gaveta => Mecanica::gaveta(),
        }
    }

    fn nome(self) -> &'static str {
        match self {
            Self::Traseira => "Simulada — Traseira",
            Self::Gaveta => "Simulada — Gaveta",
        }
    }
}

/// Escolhe o texto de uso para mostrar junto de um erro, conforme o subcomando pedido.
pub fn uso_para(args: &[String]) -> &'static str {
    match args.first().map(String::as_str) {
        Some("simular") => USO_SIMULAR,
        Some("gerar-pdf") => USO_GERAR_PDF,
        _ => USO_GERAL,
    }
}

/// Executa a simulação e devolve um relatório legível, ou uma mensagem de erro de uso.
pub fn executar(args: &[String]) -> Result<String, String> {
    let mut output = Vec::new();
    executar_com_io(args, &mut std::io::empty(), &mut output)?;
    String::from_utf8(output).map_err(|e| e.to_string())
}

fn eh_pedido_de_ajuda(valor: &str) -> bool {
    valor == "--help" || valor == "-h"
}

fn escrever(saida: &mut impl Write, texto: &str) -> Result<(), String> {
    write!(saida, "{texto}").map_err(|e| e.to_string())
}

/// Executa a simulação com entrada e saída fornecidas pelo chamador.
pub fn executar_com_io(
    args: &[String],
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> Result<(), String> {
    let primeiro = args.first().map(String::as_str);
    let segundo = args.get(1).map(String::as_str);
    if primeiro.is_some_and(eh_pedido_de_ajuda) {
        return escrever(saida, USO_GERAL);
    }
    if segundo.is_some_and(eh_pedido_de_ajuda) {
        match primeiro {
            Some("simular") => return escrever(saida, USO_SIMULAR),
            Some("gerar-pdf") => return escrever(saida, USO_GERAR_PDF),
            _ => {}
        }
    }
    match primeiro {
        Some("simular") => {}
        Some("gerar-pdf") => return gerar_pdfs(args, saida),
        Some(comando @ ("imprimir" | "calibrar")) => {
            return Err(format!(
                "o comando '{comando}' ainda não está disponível; use 'simular' ou 'gerar-pdf'"
            ));
        }
        _ => return Err(MSG_COMANDO_DESCONHECIDO.into()),
    }
    let mut paginas = None;
    let mut lote = None;
    let mut impressora = None;
    let mut guiado = false;
    let mut i = 1;
    while i < args.len() {
        let chave = &args[i];
        if chave.to_ascii_lowercase().ends_with(".pdf") || chave == "--pdf" {
            return Err(MSG_SIMULAR_PDF.into());
        }
        if chave == "--guiado" {
            guiado = true;
            i += 1;
            continue;
        }
        if i + 1 >= args.len() {
            return Err(format!("faltou valor para {chave}"));
        }
        let valor = &args[i + 1];
        match chave.as_str() {
            "--paginas" => {
                paginas = Some(
                    valor
                        .parse::<usize>()
                        .map_err(|_| "--paginas deve ser um inteiro não negativo".to_string())?,
                )
            }
            "--lote" => {
                lote = Some(
                    valor
                        .parse::<usize>()
                        .map_err(|_| "--lote deve ser um inteiro positivo".to_string())?,
                )
            }
            "--impressora" => impressora = Some(ImpressoraVirtual::analisar(valor)?),
            _ => return Err(format!("opção desconhecida: {chave}")),
        }
        i += 2;
    }
    let paginas = paginas.ok_or("informe --paginas N")?;
    let tamanho_lote = lote.ok_or("informe --lote K")?;
    let impressora = impressora.ok_or("informe --impressora traseira|gaveta")?;
    if paginas == 0 {
        return Err("--paginas deve ser maior que zero".into());
    }
    if paginas > LIMITE_PAGINAS {
        return Err(format!("--paginas não pode exceder {LIMITE_PAGINAS}"));
    }
    if tamanho_lote == 0 {
        return Err("--lote deve ser maior que zero".into());
    }
    let lotes = plano_documento(
        paginas,
        tamanho_lote,
        OrdemPassadas::ParesPrimeiro,
        impressora.perfil(),
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        saida,
        "SIMULAÇÃO — {}\n{paginas} páginas — {} lote(s)",
        impressora.nome(),
        lotes.len()
    )
    .map_err(|e| e.to_string())?;
    let mut job = Job::novo(SimBackend::novo(impressora.mecanica()), lotes.clone())
        .map_err(|e| format!("erro ao preparar simulação: {e:?}"))?;
    for (indice, plano) in lotes.iter().enumerate() {
        writeln!(
            saida,
            "\nLote {}: {} folha(s)\n  Passada 1: {}",
            indice + 1,
            plano.paginas.len() / 2,
            listar(&plano.passada1)
        )
        .map_err(|e| e.to_string())?;
        job.enviar_frente()
            .map_err(|e| format!("erro na frente: {e:?}"))?;
        let estado = job
            .atualizar()
            .map_err(|e| format!("erro ao consultar frente: {e:?}"))?;
        if estado != EstadoJob::AguardandoRecolocacao {
            return Err("a frente simulada ainda não terminou".into());
        }
        if guiado {
            write!(
                saida,
                "  Recoloque o maço simulado e pressione Enter para simular o verso: "
            )
            .map_err(|e| e.to_string())?;
            saida.flush().map_err(|e| e.to_string())?;
            let mut confirmacao = String::new();
            entrada
                .read_line(&mut confirmacao)
                .map_err(|e| format!("erro ao ler confirmação: {e}"))?;
            if !confirmacao.ends_with('\n') {
                return Err("simulação cancelada: confirmação não recebida".into());
            }
        }
        job.recolocar(Recolocacao {
            virar_maco: true,
            ..Recolocacao::default()
        })
        .map_err(|e| format!("erro na recolocação: {e:?}"))?;
        writeln!(saida, "  Passada 2: {}", listar(&plano.passada2)).map_err(|e| e.to_string())?;
        job.enviar_verso()
            .map_err(|e| format!("erro no verso: {e:?}"))?;
        job.atualizar()
            .map_err(|e| format!("erro ao consultar verso: {e:?}"))?;
        let mut folhas: Vec<_> = job
            .backend()
            .simulador()
            .pilha()
            .iter()
            .map(|f| {
                let a = f
                    .face_b
                    .and_then(|m| m.pagina)
                    .map_or_else(|| "branca".into(), |n| n.to_string());
                let b = f
                    .face_a
                    .and_then(|m| m.pagina)
                    .map_or_else(|| "branca".into(), |n| n.to_string());
                format!("{a}|{b}")
            })
            .collect();
        folhas.reverse();
        writeln!(
            saida,
            "  Folhas finais (frente|verso, de cima para baixo): {}",
            folhas.join(", ")
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn gerar_pdfs(args: &[String], saida: &mut impl Write) -> Result<(), String> {
    let entrada = args
        .get(1)
        .filter(|s| !s.starts_with("--"))
        .ok_or("informe o arquivo PDF")?;
    let (mut tamanho, mut impressora, mut diretorio) = (None, None, None);
    let mut i = 2;
    while i < args.len() {
        let chave = &args[i];
        let valor = args
            .get(i + 1)
            .ok_or_else(|| format!("faltou valor para {chave}"))?;
        match chave.as_str() {
            "--lote" if tamanho.is_none() => {
                tamanho = Some(
                    valor
                        .parse::<usize>()
                        .map_err(|_| "--lote deve ser um inteiro positivo")?,
                )
            }
            "--impressora" if impressora.is_none() => {
                impressora = Some(ImpressoraVirtual::analisar(valor)?)
            }
            "--saida" if diretorio.is_none() => diretorio = Some(std::path::PathBuf::from(valor)),
            _ => return Err(format!("opção desconhecida ou repetida: {chave}")),
        }
        i += 2;
    }
    let tamanho = tamanho.ok_or("informe --lote K")?;
    let impressora = impressora.ok_or("informe --impressora traseira|gaveta")?;
    let diretorio = diretorio.ok_or("informe --saida DIRETORIO")?;
    if !diretorio.is_dir() {
        return Err("o diretório de saída deve existir".into());
    }
    let bytes = std::fs::read(entrada).map_err(|e| format!("não foi possível ler o PDF: {e}"))?;
    let pdf = plots_pdf::PdfOrigem::carregar(&bytes).map_err(|e| e.to_string())?;
    let dimensoes = pdf.dimensoes_paginas().map_err(|e| e.to_string())?;
    let lotes = plano_lotes_por_papel(
        &dimensoes,
        tamanho,
        OrdemPassadas::ParesPrimeiro,
        impressora.perfil(),
    )
    .map_err(|e| e.to_string())?;
    let pares_mistos = pares_com_orientacao_mista(&dimensoes).map_err(|e| e.to_string())?;
    let mut arquivos = Vec::new();
    for (i, lote) in lotes.iter().enumerate() {
        let referencia = lote
            .paginas
            .iter()
            .flatten()
            .last()
            .copied()
            .ok_or("lote vazio")?;
        for (passada, paginas) in [(1, &lote.passada1), (2, &lote.passada2)] {
            let caminho = diretorio.join(format!("lote-{:04}-passada-{passada}.pdf", i + 1));
            let conteudo = pdf
                .gerar_passada(paginas, referencia)
                .map_err(|e| e.to_string())?;
            arquivos.push((caminho, conteudo));
        }
    }
    let mut criados = Vec::new();
    for (caminho, conteudo) in &arquivos {
        let resultado = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(caminho)
            .and_then(|mut f| {
                criados.push(caminho.clone());
                f.write_all(conteudo)
            });
        if let Err(e) = resultado {
            for criado in &criados {
                let _ = std::fs::remove_file(criado);
            }
            return Err(format!("não foi possível criar {}: {e}", caminho.display()));
        }
    }
    for (frente, verso) in &pares_mistos {
        writeln!(
            saida,
            "Aviso: a folha das páginas {frente} e {verso} mistura retrato e paisagem; confira o resultado antes de imprimir."
        )
        .map_err(|e| e.to_string())?;
    }
    writeln!(
        saida,
        "Gerados {} PDFs em {}. Nenhum arquivo foi enviado à impressora.",
        arquivos.len(),
        diretorio.display()
    )
    .map_err(|e| e.to_string())
}

fn listar(paginas: &[plots_core::Pagina]) -> String {
    if paginas.is_empty() {
        return "(vazia)".into();
    }
    paginas
        .iter()
        .map(|p| {
            p.numero
                .map_or_else(|| "branca".to_string(), |n| n.to_string())
        })
        .collect::<Vec<_>>()
        .join(", ")
}
