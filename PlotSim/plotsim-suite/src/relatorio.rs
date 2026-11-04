//! Relatório em HTML: uma linha por cenário, com o resultado, o motivo e as folhas finais.
//!
//! Tudo que vem de fora é escapado. O estado nunca é só cor: a linha diz "passou" ou "falhou".

use plotsim_physics::{FolhaLida, MacoLido, Motivo};
use std::fmt::Write as _;
use std::time::Duration;

/// Como terminou um cenário. "Sem cobertura" não é aprovação: a fila não tem o papel do cenário.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    Passou,
    Falhou,
    SemCobertura,
}

/// Quantos cenários terminaram em cada estado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contagem {
    pub passaram: usize,
    pub sem_cobertura: usize,
    pub falharam: usize,
}

/// Conta os estados de uma lista de cenários.
pub fn contar(resultados: &[ResultadoDoCenario]) -> Contagem {
    let com_estado = |alvo: Estado| resultados.iter().filter(|r| r.estado == alvo).count();
    Contagem {
        passaram: com_estado(Estado::Passou),
        sem_cobertura: com_estado(Estado::SemCobertura),
        falharam: com_estado(Estado::Falhou),
    }
}

/// O resultado de um cenário.
#[derive(Clone, Debug)]
pub struct ResultadoDoCenario {
    pub nome: String,
    pub estado: Estado,
    /// Por que passou ou falhou.
    pub motivo: String,
    /// Maços finais, um por fila, ou um só quando os pacotes se juntam.
    pub macos: Vec<(String, MacoLido)>,
    pub motivos: Vec<Motivo>,
    pub duracao: Duration,
}

/// Quantas folhas desenhar por maço: um documento grande não precisa de milhares de figuras.
const LIMITE_DE_FOLHAS_DESENHADAS: usize = 120;

fn escapar(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    for c in texto.chars() {
        match c {
            '&' => saida.push_str("&amp;"),
            '<' => saida.push_str("&lt;"),
            '>' => saida.push_str("&gt;"),
            '"' => saida.push_str("&quot;"),
            '\'' => saida.push_str("&#39;"),
            outro => saida.push(outro),
        }
    }
    saida
}

fn paginas_em_texto(paginas: &[u32]) -> String {
    if paginas.is_empty() {
        return String::new();
    }
    paginas
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join("+")
}

fn folha_do_motivo(motivo: &Motivo) -> Option<u32> {
    match *motivo {
        Motivo::VersoDeCabecaParaBaixo { folha }
        | Motivo::PaginaGirada { folha, .. }
        | Motivo::FolhaDeCabecaParaBaixo { folha }
        | Motivo::FrenteTrocada { folha, .. }
        | Motivo::VersoTrocado { folha, .. }
        | Motivo::FolhaAoContrario { folha }
        | Motivo::PaginaEstranha { folha, .. }
        | Motivo::FaltaPagina { folha, .. }
        | Motivo::ImpressaoSobreImpressao { folha }
        | Motivo::VersoForaDoPar { folha, .. } => Some(folha),
        Motivo::FolhaAMenos { .. } | Motivo::FolhaAMais { .. } => None,
    }
}

/// Uma folha desenhada: a frente em cheio e o verso espelhado e transparente, como visto contra a
/// luz. Se o verso está de cabeça para baixo, aparece também de cabeça para baixo.
fn desenhar_folha(saida: &mut String, numero: u32, folha: &FolhaLida, com_problema: bool) {
    let frente = paginas_em_texto(&folha.frente);
    let verso = paginas_em_texto(&folha.verso);
    let giro_do_verso = if folha.topos_na_mesma_aresta == Some(false) {
        "rotate(180 24 34) "
    } else {
        ""
    };
    let classe = if com_problema { "folha ruim" } else { "folha" };
    let selo = if com_problema {
        r##"<text class="selo" x="44" y="12" text-anchor="end">!</text>"##
    } else {
        ""
    };
    let _ = write!(
        saida,
        r##"<svg class="{classe}" viewBox="0 0 48 68" width="48" height="68" role="img" aria-label="folha {numero}: frente {frente}, verso {verso}{alerta}"><rect x="0.5" y="0.5" width="47" height="67"/><text class="verso" transform="{giro_do_verso}translate(48 0) scale(-1 1)" x="24" y="44" text-anchor="middle">{verso}</text><text class="frente" x="24" y="30" text-anchor="middle">{frente}</text>{selo}</svg>"##,
        alerta = if com_problema { ", com problema" } else { "" },
    );
}

fn desenhar_maco(saida: &mut String, nome: &str, maco: &MacoLido, motivos: &[Motivo]) {
    let _ = write!(
        saida,
        r#"<div class="maco"><div class="rotulo">{}: {} folhas</div><div class="folhas">"#,
        escapar(nome),
        maco.folhas.len()
    );
    for (indice, folha) in maco
        .folhas
        .iter()
        .take(LIMITE_DE_FOLHAS_DESENHADAS)
        .enumerate()
    {
        let numero = u32::try_from(indice).unwrap_or(u32::MAX).saturating_add(1);
        let com_problema = motivos.iter().any(|m| folha_do_motivo(m) == Some(numero));
        desenhar_folha(saida, numero, folha, com_problema);
    }
    if maco.folhas.len() > LIMITE_DE_FOLHAS_DESENHADAS {
        let _ = write!(
            saida,
            r#"<span class="corte">e mais {} folhas</span>"#,
            maco.folhas.len() - LIMITE_DE_FOLHAS_DESENHADAS
        );
    }
    saida.push_str("</div></div>");
}

const ESTILO: &str = r"
:root { color-scheme: light; }
body { margin: 24px; background: #fafaf9; color: #1c1917; font: 14px/1.45 system-ui, sans-serif; }
h1 { font-size: 20px; margin: 0 0 4px; }
.resumo { margin: 0 0 16px; }
table { border-collapse: collapse; width: 100%; background: #ffffff; }
th, td { border: 1px solid #d6d3d1; padding: 8px 10px; text-align: left; vertical-align: top; }
th { background: #f5f5f4; }
.passou { color: #166534; font-weight: 600; white-space: nowrap; }
.falhou { color: #991b1b; font-weight: 600; white-space: nowrap; }
.semcobertura { color: #44403c; font-weight: 600; white-space: nowrap; }
.motivo { max-width: 40em; }
.maco { margin: 0 0 8px; }
.rotulo { color: #44403c; margin-bottom: 4px; }
.folhas { display: flex; flex-wrap: wrap; gap: 4px; align-items: center; }
.folha rect { fill: #ffffff; stroke: #57534e; }
.folha.ruim rect { stroke: #991b1b; stroke-width: 2; }
.frente { font-size: 18px; font-weight: 600; fill: #1c1917; }
.verso { font-size: 13px; fill: #1c1917; opacity: 0.3; }
.selo { font-size: 12px; font-weight: 700; fill: #991b1b; }
.corte { color: #44403c; }
";

/// O relatório inteiro. `restou` é o que sobrou no sistema depois das remoções, e `avisos`, o que
/// o leitor do relatório precisa saber além dos cenários.
pub fn gerar_html(
    titulo: &str,
    resultados: &[ResultadoDoCenario],
    restou: &str,
    avisos: &[String],
) -> String {
    let contagem = contar(resultados);
    let total_segundos: u64 = resultados.iter().map(|r| r.duracao.as_secs()).sum();
    let mut saida = String::new();
    let _ = write!(
        saida,
        r#"<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>{titulo}</title><style>{ESTILO}</style></head><body><h1>{titulo}</h1><p class="resumo">{} de {} cenários passaram, {} sem cobertura, {} falharam, em {total_segundos} s. {}</p>"#,
        contagem.passaram,
        resultados.len(),
        contagem.sem_cobertura,
        contagem.falharam,
        escapar(restou),
        titulo = escapar(titulo),
    );
    for aviso in avisos {
        let _ = write!(saida, r#"<p class="resumo">{}</p>"#, escapar(aviso));
    }
    saida.push_str(
        "<table><thead><tr><th>Cenário</th><th>Resultado</th><th>Motivo</th><th>Folhas finais</th><th>Tempo</th></tr></thead><tbody>",
    );
    for resultado in resultados {
        let (classe, rotulo) = match resultado.estado {
            Estado::Passou => ("passou", "passou"),
            Estado::Falhou => ("falhou", "falhou"),
            Estado::SemCobertura => ("semcobertura", "sem cobertura"),
        };
        let _ = write!(
            saida,
            r#"<tr><td>{}</td><td class="{classe}">{rotulo}</td><td class="motivo">{}</td><td>"#,
            escapar(&resultado.nome),
            escapar(&resultado.motivo),
        );
        for (nome, maco) in &resultado.macos {
            desenhar_maco(&mut saida, nome, maco, &resultado.motivos);
        }
        let _ = write!(
            saida,
            "</td><td>{} s</td></tr>",
            resultado.duracao.as_secs()
        );
    }
    saida.push_str("</tbody></table></body></html>\n");
    saida
}

#[cfg(test)]
mod testes {
    use super::*;

    fn folha(frente: u32, verso: u32) -> FolhaLida {
        FolhaLida {
            frente: vec![frente],
            verso: vec![verso],
            topos_na_mesma_aresta: Some(true),
            alinhada_com_a_primeira: Some(true),
            pagina_deitada: None,
        }
    }

    fn resultado(nome: &str, estado: Estado, motivo: &str) -> ResultadoDoCenario {
        ResultadoDoCenario {
            nome: nome.to_string(),
            estado,
            motivo: motivo.to_string(),
            macos: vec![(
                "fila".to_string(),
                MacoLido {
                    folhas: vec![folha(1, 2), folha(3, 4)],
                },
            )],
            motivos: vec![Motivo::VersoDeCabecaParaBaixo { folha: 2 }],
            duracao: Duration::from_secs(3),
        }
    }

    #[test]
    fn uma_linha_por_cenario_e_o_estado_em_texto() {
        let html = gerar_html(
            "Suíte",
            &[
                resultado("a", Estado::Passou, "ok"),
                resultado("b", Estado::Falhou, "ruim"),
            ],
            "Nada restou.",
            &[],
        );
        assert_eq!(html.matches("<tr><td>").count(), 2);
        assert!(html.contains(r#"class="passou">passou"#));
        assert!(html.contains(r#"class="falhou">falhou"#));
        assert!(html.contains("1 de 2 cenários passaram"));
        assert!(html.contains("Nada restou."));
    }

    #[test]
    fn sem_cobertura_aparece_com_o_motivo_e_nao_como_passou() {
        let html = gerar_html(
            "t",
            &[resultado(
                "a",
                Estado::SemCobertura,
                "o driver da fila não tem este papel; o PlotS recusou sem enviar nada",
            )],
            "",
            &[],
        );
        assert!(html.contains(r#"class="semcobertura">sem cobertura"#));
        assert!(!html.contains(r#"class="passou">passou"#));
        assert!(html.contains("0 de 1 cenários passaram, 1 sem cobertura, 0 falharam"));
        assert!(html.contains("o driver da fila não tem este papel"));
    }

    #[test]
    fn contagem_separa_os_tres_estados() {
        let contagem = contar(&[
            resultado("a", Estado::Passou, ""),
            resultado("b", Estado::SemCobertura, ""),
            resultado("c", Estado::Falhou, ""),
        ]);
        assert_eq!(
            contagem,
            Contagem {
                passaram: 1,
                sem_cobertura: 1,
                falharam: 1
            }
        );
        assert_eq!(
            contar(&[]),
            Contagem {
                passaram: 0,
                sem_cobertura: 0,
                falharam: 0
            }
        );
    }

    #[test]
    fn texto_de_fora_e_escapado() {
        let html = gerar_html(
            "<x>",
            &[resultado("<script>", Estado::Falhou, "a & b \"c\"")],
            "<restou>",
            &["<aviso>".to_string()],
        );
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<restou>"));
        assert!(!html.contains("<aviso>"));
        assert!(html.contains("a &amp; b &quot;c&quot;"));
    }

    #[test]
    fn as_folhas_finais_saem_desenhadas_e_a_com_problema_e_marcada_sem_depender_de_cor() {
        let html = gerar_html("t", &[resultado("a", Estado::Falhou, "m")], "", &[]);
        assert_eq!(html.matches("<svg").count(), 2);
        assert!(html.contains("folha 2: frente 3, verso 4, com problema"));
        assert!(html.contains(">!</text>"));
        assert!(html.contains("scale(-1 1)"));
    }

    #[test]
    fn maco_enorme_nao_vira_milhares_de_figuras() {
        let mut r = resultado("a", Estado::Passou, "ok");
        r.macos[0].1.folhas = vec![folha(1, 2); LIMITE_DE_FOLHAS_DESENHADAS + 5];
        let html = gerar_html("t", &[r], "", &[]);
        assert_eq!(html.matches("<svg").count(), LIMITE_DE_FOLHAS_DESENHADAS);
        assert!(html.contains("e mais 5 folhas"));
    }

    #[test]
    fn folha_de_cabeca_para_baixo_tem_o_verso_girado_no_desenho() {
        let mut r = resultado("a", Estado::Falhou, "m");
        r.macos[0].1.folhas[0].topos_na_mesma_aresta = Some(false);
        assert!(gerar_html("t", &[r], "", &[]).contains("rotate(180"));
    }
}
