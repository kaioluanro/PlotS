use plots_core::{
    pares_com_orientacao_mista, plano_documento, plano_lotes_por_papel, OrdemPassadas, Pagina,
    PerfilImpressora, LIMITE_PAGINAS,
};
use plots_pdf::{FaceFinal, FolhaFinal, PapelCalibracao};
use plots_print::{
    conferir_folhas,
    job::{ErroJob, EstadoJob, Job, SimBackend},
    Folha, Marca, Mecanica, Recolocacao, Simulador, Veredito,
};
use std::io::{BufRead, Write};
use std::path::PathBuf;

/// Texto de uso geral, mostrado por `--help` e em erros sem subcomando conhecido.
pub const USO_GERAL: &str = "Uso:\n  plots simular --paginas N --lote K --impressora traseira|gaveta [--ordem pares|impares] [--simular-erro sem-virar|girado|folha-a-menos] [--guiado]\n  plots gerar-pdf ARQUIVO --lote K --impressora traseira|gaveta [--ordem pares|impares] [--folhas-finais] --saida DIRETORIO\n  plots gerar-calibracao --saida DIRETORIO [--papel a4|a5|carta|oficio|legal]\nNenhum comando envia arquivos à impressora: 'simular' só simula; 'gerar-pdf' e 'gerar-calibracao' só gravam arquivos.\n";

/// Texto de uso do subcomando `simular`.
pub const USO_SIMULAR: &str = "Uso:\n  plots simular --paginas N --lote K --impressora traseira|gaveta [--ordem pares|impares] [--simular-erro sem-virar|girado|folha-a-menos] [--guiado]\nA simulação não envia nada à impressora. Com --simular-erro, o gesto errado vale para todos os lotes; com folha-a-menos, cada lote é reimpresso desde a frente.\n";

/// Texto de uso do subcomando `gerar-pdf`.
pub const USO_GERAR_PDF: &str = "Uso:\n  plots gerar-pdf ARQUIVO --lote K --impressora traseira|gaveta [--ordem pares|impares] [--folhas-finais] --saida DIRETORIO\n--folhas-finais grava também lote-NNNN-folhas-finais.pdf, com uma página por folha: a página de número menor à esquerda e a maior à direita, em tamanho real, na ordem do maço e com a recolocação correta. As anotações não aparecem nessas folhas.\nSe o PDF tiver assinatura digital, as passadas mostram a aparência dela, mas não a contêm: servem só para imprimir e não comprovam a assinatura. Para enviar ou guardar o documento, use o arquivo original.\nOs PDFs gerados não são enviados à impressora.\n";

/// Texto de uso do subcomando `gerar-calibracao`.
pub const USO_GERAR_CALIBRACAO: &str = "Uso:\n  plots gerar-calibracao --saida DIRETORIO [--papel a4|a5|carta|oficio|legal]\nGrava calibracao.pdf com as páginas 1 a 4 e uma seta para cima em cada uma. Nada é enviado à impressora, e um arquivo existente nunca é sobrescrito.\n";

const MSG_COMANDO_DESCONHECIDO: &str =
    "comando ausente ou desconhecido; use 'simular', 'gerar-pdf' ou 'gerar-calibracao'";
const MSG_SIMULAR_PDF: &str = "'simular' não lê PDF: informe --paginas N, ou use 'gerar-pdf' para gerar as passadas de um arquivo";
const MSG_ORDEM_INVALIDA: &str = "--ordem deve ser pares ou impares";
const MSG_ORDEM_REPETIDA: &str = "--ordem foi informado mais de uma vez";
const MSG_ERRO_SIMULADO_INVALIDO: &str =
    "--simular-erro deve ser sem-virar, girado ou folha-a-menos";
const MSG_ERRO_SIMULADO_REPETIDO: &str = "--simular-erro foi informado mais de uma vez";
const MSG_AVISO_IMPARES: &str = "Ordem das passadas: ímpares primeiro.";
const MSG_FOLHAS_REPETIDA: &str = "--folhas-finais foi informado mais de uma vez";

/// Recolocação correta: vira o maço e nada mais.
const GESTO_CORRETO: Recolocacao = Recolocacao {
    virar_maco: true,
    inverter_ordem: false,
    girar_180: false,
    retirar_folhas: 0,
};

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

/// Gesto de recolocação errado que a simulação aplica de propósito.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ErroSimulado {
    SemVirar,
    Girado,
    FolhaAMenos,
}

impl ErroSimulado {
    fn analisar(valor: &str) -> Result<Self, String> {
        match valor {
            "sem-virar" => Ok(Self::SemVirar),
            "girado" => Ok(Self::Girado),
            "folha-a-menos" => Ok(Self::FolhaAMenos),
            _ => Err(MSG_ERRO_SIMULADO_INVALIDO.into()),
        }
    }

    fn nome(self) -> &'static str {
        match self {
            Self::SemVirar => "sem-virar",
            Self::Girado => "girado",
            Self::FolhaAMenos => "folha-a-menos",
        }
    }

    fn descricao(self) -> &'static str {
        match self {
            Self::SemVirar => "o maço não foi virado",
            Self::Girado => "o maço foi girado 180°",
            Self::FolhaAMenos => "uma folha saiu do maço",
        }
    }

    fn recolocacao(self) -> Recolocacao {
        match self {
            Self::SemVirar => Recolocacao {
                virar_maco: false,
                ..Recolocacao::default()
            },
            Self::Girado => Recolocacao {
                virar_maco: true,
                girar_180: true,
                ..Recolocacao::default()
            },
            Self::FolhaAMenos => Recolocacao {
                virar_maco: true,
                retirar_folhas: 1,
                ..Recolocacao::default()
            },
        }
    }
}

fn analisar_ordem(valor: &str) -> Result<OrdemPassadas, String> {
    match valor {
        "pares" => Ok(OrdemPassadas::ParesPrimeiro),
        "impares" => Ok(OrdemPassadas::ImparesPrimeiro),
        _ => Err(MSG_ORDEM_INVALIDA.into()),
    }
}

fn analisar_papel(valor: &str) -> Result<PapelCalibracao, String> {
    match valor {
        "a4" => Ok(PapelCalibracao::A4),
        "a5" => Ok(PapelCalibracao::A5),
        "carta" => Ok(PapelCalibracao::Carta),
        "oficio" => Ok(PapelCalibracao::Oficio),
        "legal" => Ok(PapelCalibracao::Legal),
        _ => Err(format!(
            "papel desconhecido: {valor}; use a4, a5, carta, oficio ou legal"
        )),
    }
}

fn nome_papel(papel: PapelCalibracao) -> &'static str {
    match papel {
        PapelCalibracao::A4 => "A4",
        PapelCalibracao::A5 => "A5",
        PapelCalibracao::Carta => "Carta",
        PapelCalibracao::Oficio => "Ofício",
        PapelCalibracao::Legal => "Legal",
    }
}

/// Escolhe o texto de uso para mostrar junto de um erro, conforme o subcomando pedido.
pub fn uso_para(args: &[String]) -> &'static str {
    match args.first().map(String::as_str) {
        Some("simular") => USO_SIMULAR,
        Some("gerar-pdf") => USO_GERAR_PDF,
        Some("gerar-calibracao") => USO_GERAR_CALIBRACAO,
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
            Some("gerar-calibracao") => return escrever(saida, USO_GERAR_CALIBRACAO),
            _ => {}
        }
    }
    match primeiro {
        Some("simular") => {}
        Some("gerar-pdf") => return gerar_pdfs(args, saida),
        Some("gerar-calibracao") => return gerar_calibracao(args, saida),
        Some(comando @ ("imprimir" | "calibrar")) => {
            return Err(format!(
                "o comando '{comando}' ainda não está disponível; use 'simular', 'gerar-pdf' ou 'gerar-calibracao'"
            ));
        }
        _ => return Err(MSG_COMANDO_DESCONHECIDO.into()),
    }
    let mut paginas = None;
    let mut lote = None;
    let mut impressora = None;
    let mut guiado = false;
    let mut ordem = None;
    let mut erro_simulado = None;
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
            "--ordem" => {
                if ordem.is_some() {
                    return Err(MSG_ORDEM_REPETIDA.into());
                }
                ordem = Some(analisar_ordem(valor)?);
            }
            "--simular-erro" => {
                if erro_simulado.is_some() {
                    return Err(MSG_ERRO_SIMULADO_REPETIDO.into());
                }
                erro_simulado = Some(ErroSimulado::analisar(valor)?);
            }
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
    let ordem = ordem.unwrap_or_default();
    let lotes = plano_documento(paginas, tamanho_lote, ordem, impressora.perfil())
        .map_err(|e| e.to_string())?;
    writeln!(
        saida,
        "SIMULAÇÃO — {}\n{paginas} páginas — {} lote(s)",
        impressora.nome(),
        lotes.len()
    )
    .map_err(|e| e.to_string())?;
    if ordem == OrdemPassadas::ImparesPrimeiro {
        writeln!(saida, "{MSG_AVISO_IMPARES}").map_err(|e| e.to_string())?;
    }
    if let Some(erro) = erro_simulado {
        writeln!(
            saida,
            "Gesto errado simulado: {} ({}).",
            erro.nome(),
            erro.descricao()
        )
        .map_err(|e| e.to_string())?;
    }
    let mut job = Job::novo(SimBackend::novo(impressora.mecanica()), lotes.clone())
        .map_err(|e| format!("erro ao preparar simulação: {e:?}"))?;
    for (indice, plano) in lotes.iter().enumerate() {
        writeln!(
            saida,
            "\nLote {}: {} folha(s)",
            indice + 1,
            plano.paginas.len() / 2
        )
        .map_err(|e| e.to_string())?;
        let mut com_erro = erro_simulado;
        loop {
            let gesto = com_erro.map_or(GESTO_CORRETO, ErroSimulado::recolocacao);
            writeln!(saida, "  Passada 1: {}", listar(&plano.passada1))
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
            job.recolocar(gesto)
                .map_err(|e| format!("erro na recolocação: {e:?}"))?;
            writeln!(saida, "  Passada 2: {}", listar(&plano.passada2))
                .map_err(|e| e.to_string())?;
            match job.enviar_verso() {
                Ok(()) => break,
                Err(ErroJob::Backend(erro)) if com_erro == Some(ErroSimulado::FolhaAMenos) => {
                    writeln!(saida, "  Erro no lote: {erro}").map_err(|e| e.to_string())?;
                    writeln!(
                        saida,
                        "  Reimprimindo o lote desde a frente, com a recolocação correta."
                    )
                    .map_err(|e| e.to_string())?;
                    job.reiniciar_lote()
                        .map_err(|e| format!("erro ao reiniciar o lote: {e:?}"))?;
                    com_erro = None;
                }
                Err(e) => return Err(format!("erro no verso: {e:?}")),
            }
        }
        job.atualizar()
            .map_err(|e| format!("erro ao consultar verso: {e:?}"))?;
        let pilha = job.backend().simulador().folhas_para_leitura();
        let veredictos = conferir_folhas(&pilha, &pares_esperados(&plano.paginas));
        let mut linhas: Vec<(String, Veredito)> =
            pilha.iter().map(rotulo_da_folha).zip(veredictos).collect();
        linhas.reverse();
        writeln!(
            saida,
            "  Folhas finais (páginas de cada folha, de cima para baixo):"
        )
        .map_err(|e| e.to_string())?;
        let mut corretas = 0usize;
        for (rotulo, veredito) in &linhas {
            match veredito {
                Veredito::Certa => {
                    corretas += 1;
                    writeln!(saida, "    {rotulo}: certa").map_err(|e| e.to_string())?;
                }
                Veredito::Errada(defeitos) => {
                    let motivos: Vec<&str> = defeitos.iter().map(|d| d.descricao()).collect();
                    writeln!(saida, "    {rotulo}: errada ({})", motivos.join("; "))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        writeln!(
            saida,
            "  Resultado do lote: {corretas} de {} folhas corretas",
            linhas.len()
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Pares de páginas esperados em cada folha do lote, na ordem do documento.
fn pares_esperados(paginas: &[Pagina]) -> Vec<(Option<usize>, Option<usize>)> {
    paginas
        .chunks(2)
        .map(|par| {
            (
                par.first().and_then(|p| p.numero),
                par.get(1).and_then(|p| p.numero),
            )
        })
        .collect()
}

/// Rótulo "menor|maior" de uma folha; a página em branco vai por último.
fn rotulo_da_folha(folha: &Folha) -> String {
    fn chave(face: Option<Marca>) -> (u8, usize, String) {
        match face {
            Some(Marca {
                pagina: Some(numero),
                ..
            }) => (0, numero, numero.to_string()),
            Some(Marca { pagina: None, .. }) => (1, 0, "branca".into()),
            None => (2, 0, "sem impressão".into()),
        }
    }
    let mut faces = [chave(folha.face_a), chave(folha.face_b)];
    faces.sort_by_key(|(grupo, numero, _)| (*grupo, *numero));
    let [primeira, segunda] = faces;
    format!("{}|{}", primeira.2, segunda.2)
}

fn gerar_pdfs(args: &[String], saida: &mut impl Write) -> Result<(), String> {
    let entrada = args
        .get(1)
        .filter(|s| !s.starts_with("--"))
        .ok_or("informe o arquivo PDF")?;
    let (mut tamanho, mut impressora, mut diretorio, mut ordem) = (None, None, None, None);
    let mut folhas_finais = false;
    let mut i = 2;
    while i < args.len() {
        let chave = &args[i];
        if chave == "--folhas-finais" {
            if folhas_finais {
                return Err(MSG_FOLHAS_REPETIDA.into());
            }
            folhas_finais = true;
            i += 1;
            continue;
        }
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
            "--ordem" => {
                if ordem.is_some() {
                    return Err(MSG_ORDEM_REPETIDA.into());
                }
                ordem = Some(analisar_ordem(valor)?);
            }
            "--saida" if diretorio.is_none() => diretorio = Some(std::path::PathBuf::from(valor)),
            _ => return Err(format!("opção desconhecida ou repetida: {chave}")),
        }
        i += 2;
    }
    let tamanho = tamanho.ok_or("informe --lote K")?;
    let impressora = impressora.ok_or("informe --impressora traseira|gaveta")?;
    let diretorio = diretorio.ok_or("informe --saida DIRETORIO")?;
    let ordem = ordem.unwrap_or_default();
    if !diretorio.is_dir() {
        return Err("o diretório de saída deve existir".into());
    }
    let bytes = std::fs::read(entrada).map_err(|e| format!("não foi possível ler o PDF: {e}"))?;
    let pdf = plots_pdf::PdfOrigem::carregar(&bytes).map_err(|e| e.to_string())?;
    let assinado = pdf.tem_assinatura_aplicada().map_err(|e| e.to_string())?;
    let dimensoes = pdf.dimensoes_paginas().map_err(|e| e.to_string())?;
    let lotes = plano_lotes_por_papel(&dimensoes, tamanho, ordem, impressora.perfil())
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
        if folhas_finais {
            let folhas =
                folhas_finais_do_lote(&lote.passada1, &lote.passada2, impressora.mecanica())?;
            let conteudo = pdf
                .gerar_folhas_finais(&folhas, referencia)
                .map_err(|e| e.to_string())?;
            let caminho = diretorio.join(format!("lote-{:04}-folhas-finais.pdf", i + 1));
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
    if ordem == OrdemPassadas::ImparesPrimeiro {
        writeln!(saida, "{MSG_AVISO_IMPARES}").map_err(|e| e.to_string())?;
    }
    if assinado {
        writeln!(
            saida,
            "Aviso: este PDF tem assinatura digital. As cópias geradas servem só para imprimir: mostram a aparência da assinatura, mas não a contêm. Para enviar ou guardar o documento, use o arquivo original."
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

/// Simula um lote com a recolocação correta e devolve as folhas finais, de cima para baixo.
fn folhas_finais_do_lote(
    frente: &[plots_core::Pagina],
    verso: &[plots_core::Pagina],
    mecanica: Mecanica,
) -> Result<Vec<FolhaFinal>, String> {
    let mut sim = Simulador::novo(mecanica);
    sim.imprimir_frente(frente)
        .map_err(|e| format!("erro na frente: {e}"))?;
    sim.recolocar(GESTO_CORRETO)
        .map_err(|e| format!("erro na recolocação: {e}"))?;
    sim.imprimir_verso(verso)
        .map_err(|e| format!("erro no verso: {e}"))?;
    Ok(sim
        .folhas_para_leitura()
        .iter()
        .rev()
        .map(|folha| ordenar_lados(folha.face_a.map(face_final), folha.face_b.map(face_final)))
        .collect())
}

/// Escolhe o lado de cada face: a página de número menor fica à esquerda e a face em branco
/// vai por último, à direita. A orientação de cada face acompanha a própria face.
fn ordenar_lados(a: Option<FaceFinal>, b: Option<FaceFinal>) -> FolhaFinal {
    let chave = |face: &Option<FaceFinal>| face.and_then(|f| f.pagina).map_or((1, 0), |n| (0, n));
    if chave(&b) < chave(&a) {
        FolhaFinal {
            frente: b,
            verso: a,
        }
    } else {
        FolhaFinal {
            frente: a,
            verso: b,
        }
    }
}

fn face_final(marca: Marca) -> FaceFinal {
    FaceFinal {
        pagina: marca.pagina,
        girada: marca.girada,
    }
}

/// Grava `calibracao.pdf` na pasta indicada, sem nunca sobrescrever um arquivo.
fn gerar_calibracao(args: &[String], saida: &mut impl Write) -> Result<(), String> {
    let (mut diretorio, mut papel) = (None, None);
    let mut i = 1;
    while i < args.len() {
        let chave = &args[i];
        let valor = args
            .get(i + 1)
            .ok_or_else(|| format!("faltou valor para {chave}"))?;
        match chave.as_str() {
            "--saida" if diretorio.is_none() => diretorio = Some(PathBuf::from(valor)),
            "--papel" if papel.is_none() => papel = Some(analisar_papel(valor)?),
            _ => return Err(format!("opção desconhecida ou repetida: {chave}")),
        }
        i += 2;
    }
    let diretorio = diretorio.ok_or("informe --saida DIRETORIO")?;
    let papel = papel.unwrap_or(PapelCalibracao::A4);
    if !diretorio.is_dir() {
        return Err("o diretório de saída deve existir".into());
    }
    let caminho = diretorio.join("calibracao.pdf");
    let bytes = plots_pdf::pdf_calibracao(papel).map_err(|e| e.to_string())?;
    let mut arquivo = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&caminho)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                format!(
                    "o arquivo {} já existe; escolha outra pasta ou remova o arquivo existente",
                    caminho.display()
                )
            } else {
                format!("não foi possível criar {}: {e}", caminho.display())
            }
        })?;
    if let Err(e) = arquivo.write_all(&bytes) {
        drop(arquivo);
        let _ = std::fs::remove_file(&caminho);
        return Err(format!(
            "não foi possível gravar {}: {e}",
            caminho.display()
        ));
    }
    writeln!(
        saida,
        "Calibração gravada em {} (papel {}, {} páginas). Nenhum arquivo foi enviado à impressora.",
        caminho.display(),
        nome_papel(papel),
        plots_pdf::PAGINAS_CALIBRACAO
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
