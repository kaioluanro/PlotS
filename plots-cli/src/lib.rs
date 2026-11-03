mod calibrar;
mod enviar;
mod filas;
mod geracao;
mod imprimir;
mod perfil;

use plots_core::{
    plano_documento, ModeloImpressora, OrdemPassadas, Pagina, PerfilImpressora, LIMITE_PAGINAS,
};
use plots_pdf::PapelCalibracao;
use plots_print::fluxo::pares_esperados;
use plots_print::{
    conferir_folhas,
    job::{ErroJob, EstadoJob, Job, SimBackend},
    Folha, Marca, Mecanica, Recolocacao, Veredito,
};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// Texto de uso geral, mostrado por `--help` e em erros sem subcomando conhecido.
pub const USO_GERAL: &str = "Uso:\n  plots simular --paginas N --lote K --impressora traseira|gaveta [--ordem pares|impares] [--simular-erro sem-virar|girado|folha-a-menos] [--guiado]\n  plots gerar-pdf ARQUIVO --lote K --impressora traseira|gaveta [--ordem pares|impares] [--folhas-finais] --saida DIRETORIO\n  plots gerar-calibracao --saida DIRETORIO [--papel a4|a5|carta|oficio|legal]\n  plots calibrar NOME --saida DIRETORIO [--papel a4|a5|carta|oficio|legal] [--perfis ARQUIVO]\n  plots imprimir ARQUIVO --impressora NOME --lote K [--arquivo-saida PASTA] [--ordem pares|impares] [--perfis ARQUIVO] [--simular --saida DIRETORIO [--folhas-finais]]\n  plots perfil NOME --modelo jato|laser [--perfis ARQUIVO]\n  plots filas [--perfis ARQUIVO]\nSó 'imprimir' sem --simular envia à impressora, com uma confirmação antes de cada envio. 'simular', 'gerar-pdf', 'gerar-calibracao', 'calibrar', 'perfil' e 'filas' só simulam, gravam arquivos ou leem o estado das filas, e 'imprimir --simular' também só simula.\n";

/// Texto de uso do subcomando `filas`.
pub const USO_FILAS: &str = "Uso:\n  plots filas [--perfis ARQUIVO]\nLista as filas do Windows, com o estado de cada uma, o driver, a porta, se é a padrão e se tem perfil salvo. As impressoras simuladas aparecem em seção própria.\nEsta listagem só lê o estado: nada é enviado a nenhuma fila, física ou virtual.\n--perfis ARQUIVO indica o arquivo de perfis; sem ele, usa o arquivo de perfis do usuário.\n";

/// Texto de uso do subcomando `imprimir`.
pub const USO_IMPRIMIR: &str = "Uso:\n  plots imprimir ARQUIVO --impressora NOME --lote K [--arquivo-saida PASTA] [--ordem pares|impares] [--perfis ARQUIVO] [--simular --saida DIRETORIO [--folhas-finais]]\nSem --simular, envia à fila NOME do Windows, lote a lote. Exige um perfil salvo por 'plots perfil' ou 'plots calibrar' e uma confirmação antes de cada envio; fim da entrada cancela. O verso só é oferecido depois de o trabalho da frente terminar ou sair da fila do spooler. Isso não garante que o papel já saiu: espere a última folha antes de recolocar o maço.\n--arquivo-saida PASTA grava cada passada em um arquivo novo dentro de PASTA, pela própria fila; foi conferido só na fila de PDF do Windows.\nO envio usa pdfium.dll, ao lado do executável ou na pasta indicada por PLOTS_PDFIUM_DIR; 'node scripts/baixar-pdfium.mjs' a obtém.\nCom --simular, grava as passadas de cada lote em --saida DIRETORIO, como 'gerar-pdf', e conduz o fluxo pelo terminal, sem enviar nada à impressora. Impressoras: 'Simulada — Traseira' e 'Simulada — Gaveta' (ou traseira e gaveta). Outro NOME usa o perfil salvo: com modelo, a mecânica e o gesto do modelo, que não foi medido naquela máquina; sem modelo, os valores da calibração. Sem perfil, o comando pede um.\n--perfis ARQUIVO indica o arquivo de perfis; sem ele, usa o arquivo de perfis do usuário.\n";

/// Texto de uso do subcomando `calibrar`.
pub const USO_CALIBRAR: &str = "Uso:\n  plots calibrar NOME --saida DIRETORIO [--papel a4|a5|carta|oficio|legal] [--perfis ARQUIVO]\nGrava calibracao.pdf em DIRETORIO, sem sobrescrever, e pergunta no terminal o que saiu. O PlotS não envia o arquivo à impressora: imprima as 4 páginas pelo programa que quiser.\nO perfil salvo para NOME é ajustado pelas respostas e gravado em ARQUIVO (ou no arquivo de perfis do usuário), sem o modelo que ele tivesse, porque os valores deixam de ser os do modelo. O resultado só vale depois de conferido em um lote impresso de verdade; a simulação não conta.\n";

/// Texto de uso do subcomando `perfil`.
pub const USO_PERFIL: &str = "Uso:\n  plots perfil NOME --modelo jato|laser [--perfis ARQUIVO]\nGrava para NOME o perfil pronto do modelo, em ARQUIVO (ou no arquivo de perfis do usuário), mantendo o apelido. A fila não precisa existir e nada é enviado à impressora.\nO modelo é um ponto de partida, não uma medição: confira com 2 ou 3 folhas. 'plots calibrar' ajusta o perfil depois e tira o modelo dele.\n";

/// Texto de uso do subcomando `simular`.
pub const USO_SIMULAR: &str = "Uso:\n  plots simular --paginas N --lote K --impressora traseira|gaveta [--ordem pares|impares] [--simular-erro sem-virar|girado|folha-a-menos] [--guiado]\nA simulação não envia nada à impressora. Com --simular-erro, o gesto errado vale para todos os lotes; com folha-a-menos, cada lote é reimpresso desde a frente.\n";

/// Texto de uso do subcomando `gerar-pdf`.
pub const USO_GERAR_PDF: &str = "Uso:\n  plots gerar-pdf ARQUIVO --lote K --impressora traseira|gaveta [--ordem pares|impares] [--folhas-finais] --saida DIRETORIO\n--folhas-finais grava também lote-NNNN-folhas-finais.pdf, com uma página por folha: a página de número menor à esquerda e a maior à direita, em tamanho real, na ordem do maço e com a recolocação correta. As anotações não aparecem nessas folhas.\nSe o PDF tiver assinatura digital, as passadas mostram a aparência dela, mas não a contêm: servem só para imprimir e não comprovam a assinatura. Para enviar ou guardar o documento, use o arquivo original.\nOs PDFs gerados não são enviados à impressora.\n";

/// Texto de uso do subcomando `gerar-calibracao`.
pub const USO_GERAR_CALIBRACAO: &str = "Uso:\n  plots gerar-calibracao --saida DIRETORIO [--papel a4|a5|carta|oficio|legal]\nGrava calibracao.pdf com as páginas 1 a 4 e uma seta para cima em cada uma. Nada é enviado à impressora, e um arquivo existente nunca é sobrescrito.\n";

const MSG_COMANDO_DESCONHECIDO: &str =
    "comando ausente ou desconhecido; use 'simular', 'gerar-pdf', 'gerar-calibracao', 'imprimir', 'calibrar', 'perfil' ou 'filas'";
const MSG_SIMULAR_PDF: &str = "'simular' não lê PDF: informe --paginas N, ou use 'gerar-pdf' para gerar as passadas de um arquivo";
const MSG_ORDEM_INVALIDA: &str = "--ordem deve ser pares ou impares";
const MSG_ORDEM_REPETIDA: &str = "--ordem foi informado mais de uma vez";
const MSG_ERRO_SIMULADO_INVALIDO: &str =
    "--simular-erro deve ser sem-virar, girado ou folha-a-menos";
const MSG_ERRO_SIMULADO_REPETIDO: &str = "--simular-erro foi informado mais de uma vez";
const MSG_SEM_VIRAR_LASER: &str = "--simular-erro sem-virar não se aplica à simulada de gaveta, que não vira o maço; use girado ou folha-a-menos";
pub(crate) const MSG_AVISO_IMPARES: &str = "Ordem das passadas: ímpares primeiro.";
/// `imprimir` sem `--simular` envia de verdade. Desligar isso exige revisar `FRASE_NAO_ENVIA` e
/// a recusa sem `--simular`; o teste `frase_sobre_o_envio_acompanha_o_envio_ligado` confere.
#[cfg(test)]
pub(crate) const ENVIO_REAL_LIBERADO: bool = true;
/// Frase da ajuda geral sobre qual comando envia à impressora.
#[cfg(test)]
pub(crate) const FRASE_NAO_ENVIA: &str = "Só 'imprimir' sem --simular envia à impressora, com uma confirmação antes de cada envio. 'simular', 'gerar-pdf', 'gerar-calibracao', 'calibrar', 'perfil' e 'filas' só simulam, gravam arquivos ou leem o estado das filas, e 'imprimir --simular' também só simula.";
/// Recusa de `imprimir` sem `--simular` fora do Windows, onde o envio real não existe.
#[cfg(not(windows))]
pub(crate) const MSG_ENVIO_INDISPONIVEL: &str =
    "o envio à impressora só está disponível no Windows; use --simular para conferir o fluxo sem imprimir";
const MSG_FOLHAS_REPETIDA: &str = "--folhas-finais foi informado mais de uma vez";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ImpressoraVirtual {
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

    /// Reconhece o atalho ou o nome de exibição de uma impressora simulada.
    pub(crate) fn de_nome(valor: &str) -> Option<Self> {
        match valor {
            "traseira" | "Simulada — Traseira" => Some(Self::Traseira),
            "gaveta" | "Simulada — Gaveta" => Some(Self::Gaveta),
            _ => None,
        }
    }

    /// Cada impressora simulada imita um modelo: a traseira, o jato; a gaveta, a laser.
    pub(crate) fn modelo(self) -> ModeloImpressora {
        match self {
            Self::Traseira => ModeloImpressora::JatoDeTinta,
            Self::Gaveta => ModeloImpressora::Laser,
        }
    }

    pub(crate) fn perfil(self) -> PerfilImpressora {
        self.modelo().perfil()
    }

    pub(crate) fn gesto(self) -> Recolocacao {
        self.modelo().gesto().into()
    }

    pub(crate) fn mecanica(self) -> Mecanica {
        Mecanica::do_modelo(self.modelo())
    }

    pub(crate) fn nome(self) -> &'static str {
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

    fn descricao(self, impressora: ImpressoraVirtual) -> &'static str {
        match (self, impressora) {
            (Self::SemVirar, _) => "o maço não foi virado",
            (Self::Girado, ImpressoraVirtual::Traseira) => {
                "o maço foi virado pela lateral, e não pela borda de cima"
            }
            (Self::Girado, ImpressoraVirtual::Gaveta) => "o maço foi girado 180°",
            (Self::FolhaAMenos, _) => "uma folha saiu do maço",
        }
    }

    /// Indica se o erro faz sentido para a impressora: na laser não há virada a omitir.
    fn aplica_a(self, impressora: ImpressoraVirtual) -> bool {
        !(self == Self::SemVirar && impressora == ImpressoraVirtual::Gaveta)
    }

    /// Recolocação errada para a impressora: a sem virar omite a virada do jato; a girada
    /// vira o jato pela lateral, sem a volta da borda de cima, e gira o maço da laser sem
    /// virar; a folha a menos usa o gesto correto e perde uma folha.
    fn recolocacao(self, impressora: ImpressoraVirtual) -> Recolocacao {
        match (self, impressora) {
            (Self::SemVirar, _) => Recolocacao::default(),
            (Self::Girado, ImpressoraVirtual::Traseira) => Recolocacao {
                virar_maco: true,
                ..Recolocacao::default()
            },
            (Self::Girado, ImpressoraVirtual::Gaveta) => Recolocacao {
                girar_180: true,
                ..Recolocacao::default()
            },
            (Self::FolhaAMenos, _) => Recolocacao {
                retirar_folhas: 1,
                ..impressora.gesto()
            },
        }
    }
}

/// Nome do modelo na saída do terminal.
pub(crate) fn nome_do_modelo(modelo: ModeloImpressora) -> &'static str {
    match modelo {
        ModeloImpressora::JatoDeTinta => "Jato de tinta",
        ModeloImpressora::Laser => "Laser",
    }
}

pub(crate) fn analisar_ordem(valor: &str) -> Result<OrdemPassadas, String> {
    match valor {
        "pares" => Ok(OrdemPassadas::ParesPrimeiro),
        "impares" => Ok(OrdemPassadas::ImparesPrimeiro),
        _ => Err(MSG_ORDEM_INVALIDA.into()),
    }
}

pub(crate) fn analisar_papel(valor: &str) -> Result<PapelCalibracao, String> {
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

pub(crate) fn nome_papel(papel: PapelCalibracao) -> &'static str {
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
        Some("imprimir") => USO_IMPRIMIR,
        Some("calibrar") => USO_CALIBRAR,
        Some("filas") => USO_FILAS,
        Some("perfil") => USO_PERFIL,
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
            Some("imprimir") => return escrever(saida, USO_IMPRIMIR),
            Some("calibrar") => return escrever(saida, USO_CALIBRAR),
            Some("filas") => return escrever(saida, USO_FILAS),
            Some("perfil") => return escrever(saida, USO_PERFIL),
            _ => {}
        }
    }
    match primeiro {
        Some("simular") => {}
        Some("gerar-pdf") => return gerar_pdfs(args, saida),
        Some("gerar-calibracao") => return gerar_calibracao(args, saida),
        Some("imprimir") => return imprimir::executar(args, entrada, saida),
        Some("calibrar") => return calibrar::executar(args, entrada, saida),
        Some("filas") => return filas::executar(args, saida),
        Some("perfil") => return perfil::executar(args, saida),
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
    if erro_simulado.is_some_and(|erro| !erro.aplica_a(impressora)) {
        return Err(MSG_SEM_VIRAR_LASER.into());
    }
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
            erro.descricao(impressora)
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
            let gesto = match com_erro {
                Some(erro) => erro.recolocacao(impressora),
                None => impressora.gesto(),
            };
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
        let (corretas, total) = relatar_folhas(saida, &plano.paginas, &pilha)?;
        writeln!(
            saida,
            "  Resultado do lote: {corretas} de {total} folhas corretas"
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Escreve a conferência de cada folha final do lote, de cima para baixo, e devolve
/// (corretas, total de folhas).
pub(crate) fn relatar_folhas(
    saida: &mut impl Write,
    paginas: &[Pagina],
    pilha: &[Folha],
) -> Result<(usize, usize), String> {
    let veredictos = conferir_folhas(pilha, &pares_esperados(paginas));
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
    Ok((corretas, linhas.len()))
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
    let opcoes = geracao::Opcoes {
        tamanho,
        ordem,
        perfil: impressora.perfil(),
        mecanica: impressora.mecanica(),
        gesto: impressora.gesto(),
        folhas_finais,
    };
    let documento = geracao::Documento::ler(Path::new(entrada))?;
    let plano = geracao::planejar(&documento, opcoes.tamanho, opcoes.ordem, opcoes.perfil)?;
    let arquivos = geracao::gerar_arquivos(&documento, &plano, &opcoes, &diretorio)?;
    geracao::gravar_sem_sobrescrever(&arquivos)?;
    geracao::escrever_avisos(&documento, &plano, ordem, saida)?;
    writeln!(
        saida,
        "Gerados {} PDFs em {}. Nenhum arquivo foi enviado à impressora.",
        arquivos.len(),
        diretorio.display()
    )
    .map_err(|e| e.to_string())
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
    geracao::criar_novo(&caminho, &bytes)?;
    writeln!(
        saida,
        "Calibração gravada em {} (papel {}, {} páginas). Nenhum arquivo foi enviado à impressora.",
        caminho.display(),
        nome_papel(papel),
        plots_pdf::PAGINAS_CALIBRACAO
    )
    .map_err(|e| e.to_string())
}

pub(crate) fn listar(paginas: &[plots_core::Pagina]) -> String {
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
