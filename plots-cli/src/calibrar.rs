//! Comando `calibrar`: grava a folha de teste, lê as duas respostas e guarda o perfil da fila.

use crate::geracao;
use plots_core::{ajustar_perfil_por_calibracao, PerfilImpressora, ViradaMaco};
use plots_pdf::PapelCalibracao;
use plots_perfis::{EntradaPerfil, ErroPerfis, Perfis};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

const PERGUNTA_VERSO: &str = "O verso da folha 1 é o 2? (s/n): ";
const PERGUNTA_INVERTIDA: &str = "O 2 está de cabeça para baixo? (s/n): ";
pub(crate) const MSG_RESPOSTA_INVALIDA: &str = "Resposta inválida: responda s ou n.";
/// Também usada no texto de uso: a ajuda de `calibrar` deve trazer esta mesma frase.
pub(crate) const MSG_RESULTADO_SO_VALE_EM_LOTE_REAL: &str =
    "O resultado só vale depois de conferido em um lote impresso de verdade; a simulação não conta.";

/// Recusa de quem pede um perfil para uma impressora simulada, que já vem com o seu.
pub(crate) fn msg_impressora_simulada(fila: &str) -> String {
    format!("'{fila}' é uma impressora simulada e não precisa de calibração")
}

/// Caminho do arquivo de perfis: o informado, ou o padrão do usuário.
pub(crate) fn caminho_perfis(informado: Option<PathBuf>) -> Result<PathBuf, String> {
    match informado {
        Some(caminho) => Ok(caminho),
        None => plots_perfis::caminho_padrao().map_err(|e| mensagem_perfis(&e)),
    }
}

/// Lê o arquivo de perfis. Ausente resulta em conjunto vazio; nada é alterado.
pub(crate) fn carregar_perfis(caminho: &Path) -> Result<Perfis, String> {
    plots_perfis::carregar(caminho).map_err(|e| mensagem_perfis(&e))
}

/// Grava o conjunto de perfis de forma atômica.
pub(crate) fn salvar_perfis(caminho: &Path, perfis: &Perfis) -> Result<(), String> {
    plots_perfis::salvar(caminho, perfis).map_err(|e| mensagem_perfis(&e))
}

/// Mensagem em português para o usuário. Detalhes de TOML, que vêm em inglês e podem
/// conter nomes de filas, ficam de fora.
fn mensagem_perfis(erro: &ErroPerfis) -> String {
    match erro {
        ErroPerfis::TomlInvalido { caminho, .. } => format!(
            "o arquivo de perfis {} não é um TOML válido; ele não foi alterado",
            caminho.display()
        ),
        ErroPerfis::Serializacao { .. } => {
            "não foi possível preparar os perfis para gravação; nenhum arquivo foi alterado"
                .to_string()
        }
        outro => outro.to_string(),
    }
}

/// Executa `calibrar NOME --saida DIRETORIO [--papel P] [--perfis ARQUIVO]`.
pub(crate) fn executar(
    args: &[String],
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> Result<(), String> {
    let fila = args
        .get(1)
        .filter(|s| !s.starts_with("--") && !s.trim().is_empty())
        .ok_or("informe o nome da fila")?;
    let (mut diretorio, mut papel, mut perfis) = (None, None, None);
    let mut i = 2;
    while i < args.len() {
        let chave = &args[i];
        let valor = args
            .get(i + 1)
            .ok_or_else(|| format!("faltou valor para {chave}"))?;
        match chave.as_str() {
            "--saida" if diretorio.is_none() => diretorio = Some(PathBuf::from(valor)),
            "--papel" if papel.is_none() => papel = Some(crate::analisar_papel(valor)?),
            "--perfis" if perfis.is_none() => perfis = Some(PathBuf::from(valor)),
            _ => return Err(format!("opção desconhecida ou repetida: {chave}")),
        }
        i += 2;
    }
    let diretorio = diretorio.ok_or("informe --saida DIRETORIO")?;
    let papel = papel.unwrap_or(PapelCalibracao::A4);
    if crate::ImpressoraVirtual::de_nome(fila).is_some() {
        return Err(msg_impressora_simulada(fila));
    }
    if !diretorio.is_dir() {
        return Err("o diretório de saída deve existir".into());
    }
    let caminho_perfis = caminho_perfis(perfis)?;
    let mut lidos = carregar_perfis(&caminho_perfis)?;
    let arquivo = diretorio.join("calibracao.pdf");
    let bytes = plots_pdf::pdf_calibracao(papel).map_err(|e| e.to_string())?;
    geracao::criar_novo(&arquivo, &bytes)?;
    writeln!(
        saida,
        "Calibração de {fila} gravada em {} (papel {}, {} páginas).",
        arquivo.display(),
        crate::nome_papel(papel),
        plots_pdf::PAGINAS_CALIBRACAO
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        saida,
        "O PlotS não envia o arquivo: imprima as 4 páginas pelo programa que quiser e recoloque o maço do jeito de costume."
    )
    .map_err(|e| e.to_string())?;
    let verso_certo = perguntar_sim_nao(entrada, saida, &diretorio, PERGUNTA_VERSO)?;
    let invertida = perguntar_sim_nao(entrada, saida, &diretorio, PERGUNTA_INVERTIDA)?;
    let base = lidos.obter(fila).map(|e| e.perfil).unwrap_or_default();
    let ajustado = ajustar_perfil_por_calibracao(base, verso_certo, invertida);
    let mut nova: EntradaPerfil = lidos.obter(fila).cloned().unwrap_or_default();
    nova.perfil = ajustado;
    // Os valores deixam de ser os do modelo, mesmo que as respostas não mudem nenhum deles.
    nova.modelo = None;
    lidos.inserir(fila, nova);
    salvar_perfis(&caminho_perfis, &lidos)?;
    writeln!(
        saida,
        "Perfil de {fila} gravado em {}:",
        caminho_perfis.display()
    )
    .map_err(|e| e.to_string())?;
    for linha in descrever_perfil(&ajustado) {
        writeln!(saida, "  {linha}").map_err(|e| e.to_string())?;
    }
    writeln!(saida, "{MSG_RESULTADO_SO_VALE_EM_LOTE_REAL}").map_err(|e| e.to_string())
}

/// Pergunta até receber s ou n. Fim da entrada cancela, sem gravar nada.
fn perguntar_sim_nao(
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
    diretorio: &Path,
    pergunta: &str,
) -> Result<bool, String> {
    loop {
        write!(saida, "{pergunta}").map_err(|e| e.to_string())?;
        saida.flush().map_err(|e| e.to_string())?;
        let mut resposta = String::new();
        let lidos = entrada
            .read_line(&mut resposta)
            .map_err(|e| format!("erro ao ler a resposta: {e}"))?;
        if lidos == 0 {
            return Err(format!(
                "calibração cancelada: calibracao.pdf ficou em {}; nenhum perfil foi gravado",
                diretorio.display()
            ));
        }
        match resposta.trim().to_lowercase().as_str() {
            "s" => return Ok(true),
            "n" => return Ok(false),
            _ => writeln!(saida, "{MSG_RESPOSTA_INVALIDA}").map_err(|e| e.to_string())?,
        }
    }
}

/// Descreve o perfil em palavras, uma linha por parâmetro.
pub(crate) fn descrever_perfil(perfil: &PerfilImpressora) -> Vec<String> {
    let ordem = |inversa: bool| {
        if inversa {
            "da última para a primeira"
        } else {
            "da primeira para a última"
        }
    };
    let giro = if perfil.verso_girar_180 {
        "páginas giradas 180°"
    } else {
        "páginas sem giro"
    };
    let recolocacao = match perfil.gesto_recolocar.virada {
        ViradaMaco::NaoVirar => "não virar o maço",
        ViradaMaco::PelaLateral => "virar o maço",
        ViradaMaco::PelaBordaDeCima => "virar o maço pela borda de cima",
    };
    let borda = match perfil.gesto_recolocar.borda_primeiro {
        plots_core::BordaEntrada::Curta => "curta",
        plots_core::BordaEntrada::Longa => "longa",
    };
    vec![
        format!("Frente: {}", ordem(perfil.passada1_inversa)),
        format!("Verso: {}", ordem(perfil.passada2_inversa)),
        format!("Verso: {giro}"),
        format!("Recolocação: {recolocacao}"),
        format!("Borda que entra primeiro: {borda}"),
    ]
}

#[cfg(test)]
mod testes {
    use super::*;
    use plots_core::{BordaEntrada, GestoRecolocar};

    #[test]
    fn descreve_o_perfil_padrao_em_palavras() {
        assert_eq!(
            descrever_perfil(&PerfilImpressora::default()),
            [
                "Frente: da primeira para a última",
                "Verso: da primeira para a última",
                "Verso: páginas sem giro",
                "Recolocação: virar o maço",
                "Borda que entra primeiro: curta",
            ]
        );
    }

    #[test]
    fn descreve_a_virada_pela_borda_de_cima() {
        let linhas = descrever_perfil(&plots_core::ModeloImpressora::JatoDeTinta.perfil());
        assert!(
            linhas.contains(&"Recolocação: virar o maço pela borda de cima".to_owned()),
            "{linhas:?}"
        );
    }

    #[test]
    fn descreve_os_valores_invertidos_em_palavras() {
        let perfil = PerfilImpressora {
            passada1_inversa: true,
            passada2_inversa: true,
            verso_girar_180: true,
            gesto_recolocar: GestoRecolocar {
                virada: ViradaMaco::NaoVirar,
                borda_primeiro: BordaEntrada::Longa,
            },
        };
        assert_eq!(
            descrever_perfil(&perfil),
            [
                "Frente: da última para a primeira",
                "Verso: da última para a primeira",
                "Verso: páginas giradas 180°",
                "Recolocação: não virar o maço",
                "Borda que entra primeiro: longa",
            ]
        );
    }
}
