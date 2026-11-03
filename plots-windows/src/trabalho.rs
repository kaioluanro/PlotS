//! Tradução do estado de um trabalho no spooler, sem consultar o sistema.
//!
//! Limite conhecido: a confirmação do spooler só garante a entrega ao destino seguinte, não a
//! saída do papel. Um trabalho que some da fila depois de enviado também é tratado como
//! terminado, porque o sistema não guarda o resultado de um trabalho que já saiu.

use crate::estado::{Condicao, Estado};
use plots_print::fluxo::CondicaoDoAviso;
use plots_print::job::EstadoEnvio;

/// Bits de `JOB_STATUS_*` da documentação do winspool.
const ERRO: u32 = 0x0000_0002;
const EXCLUINDO: u32 = 0x0000_0004;
const SPOOLING: u32 = 0x0000_0008;
const IMPRIMINDO: u32 = 0x0000_0010;
const OFFLINE: u32 = 0x0000_0020;
const SEM_PAPEL: u32 = 0x0000_0040;
const IMPRESSO: u32 = 0x0000_0080;
const EXCLUIDO: u32 = 0x0000_0100;
const BLOQUEADO: u32 = 0x0000_0200;
const INTERVENCAO: u32 = 0x0000_0400;
const COMPLETO: u32 = 0x0000_1000;
const RETIDO: u32 = 0x0000_2000;

const TEXTO_ATOLADO: &str = "o papel atolou na impressora";
const TEXTO_SEM_PAPEL: &str = "a impressora está sem papel";
const TEXTO_SEM_RESPOSTA: &str = "a impressora não responde";
const TEXTO_ATENCAO: &str = "a impressora pede atenção";

/// Quantos caracteres do texto do driver o programa olha e quantos mostra. O texto vem de terceiros
/// e pode ter qualquer tamanho.
const MAXIMO_LIDO_DO_DRIVER: usize = 4096;
const MAXIMO_DA_MENSAGEM_DO_DRIVER: usize = 200;

/// Bits que impedem o trabalho de continuar, com o texto de cada um.
const FALHAS: [(u32, &str); 2] = [
    (BLOQUEADO, "o trabalho está bloqueado na impressora"),
    (ERRO, "o trabalho deu erro"),
];

/// Resultado da leitura de um trabalho.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SituacaoTrabalho {
    pub estado: EstadoEnvio,
    /// Por que falhou ou, com o trabalho em andamento, o que a impressora espera que alguém
    /// resolva; vazio nos outros casos.
    pub motivo: Option<String>,
    /// A condição mais urgente do aviso, quando ela é uma das que têm orientação própria.
    pub condicao: Option<CondicaoDoAviso>,
}

fn textos_ligados(lista: &[(u32, &'static str)], status: u32) -> Vec<&'static str> {
    lista
        .iter()
        .filter(|(bit, _)| status & bit != 0)
        .map(|(_, texto)| *texto)
        .collect()
}

/// O que a pessoa precisa resolver, do mais urgente ao menos. O atolamento só aparece no estado da
/// fila: o trabalho não tem bit para ele. Bit genérico ou desconhecido da fila não vira aviso.
fn avisos_ligados(
    status: u32,
    fila: Option<&Estado>,
) -> Vec<(&'static str, Option<CondicaoDoAviso>)> {
    let tem = |procuradas: &[Condicao]| {
        fila.is_some_and(|fila| fila.condicoes().iter().any(|c| procuradas.contains(c)))
    };
    let lista = [
        (
            tem(&[Condicao::Atolada]),
            TEXTO_ATOLADO,
            Some(CondicaoDoAviso::Atolado),
        ),
        (
            status & SEM_PAPEL != 0 || tem(&[Condicao::SemPapel]),
            TEXTO_SEM_PAPEL,
            Some(CondicaoDoAviso::SemPapel),
        ),
        (
            status & OFFLINE != 0
                || tem(&[
                    Condicao::Offline,
                    Condicao::Indisponivel,
                    Condicao::ServidorOffline,
                ]),
            TEXTO_SEM_RESPOSTA,
            Some(CondicaoDoAviso::SemResposta),
        ),
        (status & INTERVENCAO != 0, TEXTO_ATENCAO, None),
    ];
    lista
        .into_iter()
        .filter_map(|(ligado, texto, condicao)| ligado.then_some((texto, condicao)))
        .collect()
}

/// O trabalho já terminou ou está saindo da fila: o estado da fila não muda mais nada nele.
pub(crate) fn trabalho_terminou(status: u32) -> bool {
    status & (EXCLUINDO | EXCLUIDO) != 0 || saida_concluida(status)
}

/// A saída do trabalho está pronta. Retido sozinho não basta: há driver que liga esse bit antes de
/// gravar a saída, junto com imprimindo ou gravando.
fn saida_concluida(status: u32) -> bool {
    status & (IMPRESSO | COMPLETO) != 0
        || (status & RETIDO != 0 && status & (IMPRIMINDO | SPOOLING) == 0)
}

/// Traduz o campo `Status` de `JOB_INFO_2W`, sem olhar a fila.
pub fn traduzir_status(status: u32) -> SituacaoTrabalho {
    traduzir_com_fila(status, None)
}

/// Traduz o campo `Status` de `JOB_INFO_2W` junto com o estado da fila, quando ele pôde ser lido.
///
/// Sem papel, atolamento, falta de resposta e pedido de atenção vêm primeiro: o trabalho pode
/// voltar a andar sozinho quando alguém resolver, e por isso segue em andamento, com o motivo,
/// mesmo com o bit de erro que muitos drivers ligam junto. Depois vem a falha, que vence os demais
/// bits porque um trabalho com erro pode continuar marcado como imprimindo.
///
/// O estado da fila só conta enquanto o trabalho não terminou: bandeja vazia depois da última folha
/// é o normal antes de recolocar o maço, e não pode segurar a confirmação da frente.
pub fn traduzir_com_fila(status: u32, fila: Option<&Estado>) -> SituacaoTrabalho {
    let terminou = trabalho_terminou(status);
    let falhas = textos_ligados(&FALHAS, status);
    let avisos = avisos_ligados(status, fila.filter(|_| !terminou));
    if !avisos.is_empty() {
        let condicao = avisos.iter().find_map(|(_, condicao)| *condicao);
        let textos = avisos.iter().map(|(texto, _)| *texto).collect::<Vec<_>>();
        return SituacaoTrabalho {
            estado: EstadoEnvio::EmAndamento,
            motivo: Some([textos, falhas].concat().join("; ")),
            condicao,
        };
    }
    if !falhas.is_empty() {
        return falhou(falhas.join("; "));
    }
    if saida_concluida(status) {
        return SituacaoTrabalho {
            estado: EstadoEnvio::Concluido,
            motivo: None,
            condicao: None,
        };
    }
    // Há driver cujo trabalho bem-sucedido termina assim: retido, saindo da fila e ainda imprimindo.
    // Isso não decide nada; o trabalho some logo depois, e quem acompanha trata o sumiço.
    let saindo_retido =
        status & (RETIDO | EXCLUINDO | EXCLUIDO | IMPRIMINDO) == (RETIDO | EXCLUINDO | IMPRIMINDO);
    // Excluir sem ter terminado quer dizer que alguém tirou o trabalho da fila.
    if status & (EXCLUINDO | EXCLUIDO) != 0 && !saindo_retido {
        return falhou("o trabalho foi excluído da fila antes de terminar".to_string());
    }
    // Os demais bits, inclusive pausado, só pedem para esperar: quem pausou pode retomar.
    SituacaoTrabalho {
        estado: EstadoEnvio::EmAndamento,
        motivo: None,
        condicao: None,
    }
}

/// Caracteres que não aparecem, mas mudam o que se lê: direção do texto, largura zero e afins.
fn eh_invisivel(letra: char) -> bool {
    matches!(
        letra,
        '\u{00AD}'
            | '\u{061C}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{E0000}'..='\u{E007F}'
    )
}

/// Pula o resto de uma sequência de controle de terminal, depois do caractere de escape.
fn pular_escape(letras: &mut impl Iterator<Item = char>) {
    match letras.next() {
        Some('[') => pular_ate_o_fim_do_csi(letras),
        Some(']') => {
            let _ = letras.find(|letra| matches!(letra, '\u{7}' | '\u{1b}' | '\u{9c}'));
        }
        _ => {}
    }
}

fn pular_ate_o_fim_do_csi(letras: &mut impl Iterator<Item = char>) {
    let _ = letras.find(|letra| matches!(letra, '@'..='~'));
}

/// Deixa o texto do driver pronto para aparecer em uma linha: sem marcação, sem sequência de
/// terminal, sem caractere invisível, em uma linha só e de tamanho limitado. Texto que não sobra
/// nada não é mensagem.
pub(crate) fn limpar_mensagem_do_driver(texto: &str) -> Option<String> {
    let mut limpo = String::new();
    let mut letras = texto.chars().take(MAXIMO_LIDO_DO_DRIVER).peekable();
    while let Some(letra) = letras.next() {
        match letra {
            '\u{1b}' => pular_escape(&mut letras),
            '\u{9b}' => pular_ate_o_fim_do_csi(&mut letras),
            '<' if letras.peek().is_some_and(|proxima| {
                proxima.is_alphabetic() || matches!(proxima, '/' | '!' | '?')
            }) =>
            {
                let _ = letras.find(|outra| *outra == '>');
                limpo.push(' ');
            }
            '<' | '>' => {}
            _ if letra.is_whitespace() => limpo.push(' '),
            _ if letra.is_control() || eh_invisivel(letra) => {}
            _ => limpo.push(letra),
        }
    }
    let palavras = limpo.split_whitespace().collect::<Vec<_>>().join(" ");
    if palavras.is_empty() {
        return None;
    }
    if palavras.chars().count() <= MAXIMO_DA_MENSAGEM_DO_DRIVER {
        return Some(palavras);
    }
    let cortada: String = palavras
        .chars()
        .take(MAXIMO_DA_MENSAGEM_DO_DRIVER - 1)
        .collect();
    Some(format!("{}…", cortada.trim_end()))
}

/// O que vai em cada campo depois de ler o trabalho: no aviso, a mensagem do driver fica em campo
/// próprio; na falha, que não tem outra linha para mostrar, ela segue junto do motivo.
pub(crate) fn motivo_e_mensagem(
    estado: EstadoEnvio,
    motivo: Option<String>,
    texto_do_driver: &str,
) -> (Option<String>, Option<String>) {
    let Some(motivo) = motivo else {
        return (None, None);
    };
    let mensagem = limpar_mensagem_do_driver(texto_do_driver);
    match (estado, mensagem) {
        (EstadoEnvio::EmAndamento, mensagem) => (Some(motivo), mensagem),
        (_, Some(mensagem)) => (Some(format!("{motivo} ({mensagem})")), None),
        (_, None) => (Some(motivo), None),
    }
}

fn falhou(motivo: String) -> SituacaoTrabalho {
    SituacaoTrabalho {
        estado: EstadoEnvio::Falhou,
        motivo: Some(motivo),
        condicao: None,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn sem_bits_e_durante_a_impressao_continua_em_andamento() {
        // Fila de espera, gravando, imprimindo, pausado, reiniciando e desenhando no cliente.
        for status in [0, 0x8, 0x10, 0x8 | 0x10, 0x1, 0x800, 0x4000] {
            assert_eq!(
                traduzir_status(status).estado,
                EstadoEnvio::EmAndamento,
                "{status:#x}"
            );
        }
    }

    #[test]
    fn impresso_completo_ou_retido_terminam() {
        for status in [
            IMPRESSO,
            COMPLETO,
            RETIDO,
            IMPRESSO | EXCLUINDO,
            COMPLETO | EXCLUIDO,
        ] {
            let situacao = traduzir_status(status);
            assert_eq!(situacao.estado, EstadoEnvio::Concluido, "{status:#x}");
            assert_eq!(situacao.motivo, None);
        }
    }

    #[test]
    fn erro_e_bloqueio_viram_falha_com_motivo() {
        for (status, trecho) in [(ERRO, "erro"), (BLOQUEADO, "bloqueado")] {
            let situacao = traduzir_status(status);
            assert_eq!(situacao.estado, EstadoEnvio::Falhou, "{status:#x}");
            assert!(situacao.motivo.unwrap().contains(trecho), "{status:#x}");
        }
        assert_eq!(traduzir_status(ERRO | 0x10).estado, EstadoEnvio::Falhou);
        assert_eq!(traduzir_status(ERRO | IMPRESSO).estado, EstadoEnvio::Falhou);
    }

    #[test]
    fn papel_offline_e_atencao_sao_aviso_e_o_trabalho_continua_em_andamento() {
        for (status, trecho) in [
            (SEM_PAPEL, "sem papel"),
            (OFFLINE, "não responde"),
            (INTERVENCAO, "atenção"),
        ] {
            let situacao = traduzir_status(status);
            assert_eq!(situacao.estado, EstadoEnvio::EmAndamento, "{status:#x}");
            assert!(situacao.motivo.unwrap().contains(trecho), "{status:#x}");
        }
    }

    #[test]
    fn aviso_vence_quando_o_trabalho_tambem_parece_terminado_ou_com_erro() {
        for status in [SEM_PAPEL | 0x10 | IMPRESSO, SEM_PAPEL | ERRO] {
            let situacao = traduzir_status(status);
            assert_eq!(situacao.estado, EstadoEnvio::EmAndamento, "{status:#x}");
            assert!(situacao.motivo.unwrap().contains("sem papel"));
        }
        let duas = traduzir_status(SEM_PAPEL | OFFLINE).motivo.unwrap();
        assert!(
            duas.contains("sem papel") && duas.contains("não responde"),
            "{duas}"
        );
    }

    #[test]
    fn exclusao_antes_de_terminar_e_falha() {
        for status in [EXCLUINDO, EXCLUIDO, EXCLUINDO | 0x10] {
            let situacao = traduzir_status(status);
            assert_eq!(situacao.estado, EstadoEnvio::Falhou, "{status:#x}");
            assert!(situacao.motivo.unwrap().contains("excluído"));
        }
    }

    #[test]
    fn retido_saindo_da_fila_e_imprimindo_espera_o_sumico() {
        let status = RETIDO | EXCLUINDO | IMPRIMINDO;
        let situacao = traduzir_status(status);
        assert_eq!(situacao.estado, EstadoEnvio::EmAndamento);
        assert_eq!(situacao.motivo, None);
        assert_eq!(situacao.condicao, None);
        // Saindo da fila, a bandeja vazia depois da última folha não vira aviso.
        let situacao = com_fila(status, FILA_SEM_PAPEL);
        assert_eq!(situacao.estado, EstadoEnvio::EmAndamento);
        assert_eq!(situacao.motivo, None);
        // Com erro ligado, a falha continua valendo.
        assert_eq!(traduzir_status(status | ERRO).estado, EstadoEnvio::Falhou);
        // Já excluído de vez, ou sem o retido, continua sendo falha.
        assert_eq!(
            traduzir_status(status | EXCLUIDO).estado,
            EstadoEnvio::Falhou
        );
        assert_eq!(
            traduzir_status(EXCLUINDO | IMPRIMINDO).estado,
            EstadoEnvio::Falhou
        );
    }

    /// Bits de `PRINTER_STATUS_*` usados nos testes abaixo.
    const FILA_ATOLADA: u32 = 0x8;
    const FILA_SEM_PAPEL: u32 = 0x10;
    const FILA_OFFLINE: u32 = 0x80;
    const FILA_INDISPONIVEL: u32 = 0x1000;
    const FILA_SERVIDOR_OFFLINE: u32 = 0x0200_0000;

    fn com_fila(status: u32, fila: u32) -> SituacaoTrabalho {
        traduzir_com_fila(status, Some(&Estado::de_bits(fila)))
    }

    #[test]
    fn retido_ainda_imprimindo_ou_gravando_nao_terminou() {
        for status in [RETIDO | IMPRIMINDO, RETIDO | SPOOLING] {
            assert!(!trabalho_terminou(status), "{status:#x}");
            let situacao = traduzir_status(status);
            assert_eq!(situacao.estado, EstadoEnvio::EmAndamento, "{status:#x}");
            assert_eq!(situacao.motivo, None, "{status:#x}");
        }
    }

    #[test]
    fn retido_imprimindo_com_a_fila_atolada_mantem_o_motivo() {
        let situacao = com_fila(RETIDO | IMPRIMINDO, FILA_ATOLADA);
        assert_eq!(situacao.estado, EstadoEnvio::EmAndamento);
        assert_eq!(situacao.motivo.as_deref(), Some(TEXTO_ATOLADO));
        assert_eq!(situacao.condicao, Some(CondicaoDoAviso::Atolado));
    }

    #[test]
    fn retido_impresso_ou_sozinho_continua_concluido() {
        for status in [RETIDO | IMPRESSO, RETIDO, RETIDO | IMPRESSO | IMPRIMINDO] {
            assert!(trabalho_terminou(status), "{status:#x}");
            assert_eq!(
                traduzir_status(status).estado,
                EstadoEnvio::Concluido,
                "{status:#x}"
            );
        }
    }

    #[test]
    fn a_condicao_vem_da_mais_urgente_e_pedido_de_atencao_nao_tem_condicao() {
        let condicao = |status, fila| com_fila(status, fila).condicao;
        assert_eq!(
            condicao(IMPRIMINDO, FILA_ATOLADA | FILA_SEM_PAPEL | FILA_OFFLINE),
            Some(CondicaoDoAviso::Atolado)
        );
        assert_eq!(
            condicao(SEM_PAPEL, FILA_OFFLINE),
            Some(CondicaoDoAviso::SemPapel)
        );
        assert_eq!(condicao(OFFLINE, 0), Some(CondicaoDoAviso::SemResposta));
        assert_eq!(
            condicao(ERRO, FILA_OFFLINE),
            Some(CondicaoDoAviso::SemResposta)
        );
        assert_eq!(condicao(INTERVENCAO, 0), None);
        assert_eq!(
            condicao(INTERVENCAO, FILA_SEM_PAPEL),
            Some(CondicaoDoAviso::SemPapel)
        );
        assert_eq!(condicao(IMPRESSO, FILA_ATOLADA), None);
        assert_eq!(condicao(ERRO, 0), None);
    }

    #[test]
    fn cada_condicao_da_fila_vira_o_seu_aviso_com_o_trabalho_em_andamento() {
        for (fila, texto) in [
            (FILA_ATOLADA, TEXTO_ATOLADO),
            (FILA_SEM_PAPEL, TEXTO_SEM_PAPEL),
            (FILA_OFFLINE, TEXTO_SEM_RESPOSTA),
            (FILA_INDISPONIVEL, TEXTO_SEM_RESPOSTA),
            (FILA_SERVIDOR_OFFLINE, TEXTO_SEM_RESPOSTA),
        ] {
            let situacao = com_fila(IMPRIMINDO, fila);
            assert_eq!(situacao.estado, EstadoEnvio::EmAndamento, "{fila:#x}");
            assert_eq!(situacao.motivo.as_deref(), Some(texto), "{fila:#x}");
        }
    }

    #[test]
    fn usar_impressora_offline_conta_como_sem_resposta() {
        let fila = Estado::de_status_e_atributos(0, 0x400);
        assert_eq!(
            traduzir_com_fila(0, Some(&fila)).motivo.as_deref(),
            Some(TEXTO_SEM_RESPOSTA)
        );
    }

    #[test]
    fn atolamento_vem_antes_e_o_bit_igual_no_trabalho_e_na_fila_nao_repete() {
        let motivo = com_fila(SEM_PAPEL, FILA_ATOLADA | FILA_SEM_PAPEL)
            .motivo
            .unwrap();
        assert_eq!(motivo, format!("{TEXTO_ATOLADO}; {TEXTO_SEM_PAPEL}"));
    }

    #[test]
    fn erro_do_trabalho_com_papel_atolado_na_fila_e_aviso_e_nao_falha() {
        let situacao = com_fila(ERRO | IMPRIMINDO, FILA_ATOLADA);
        assert_eq!(situacao.estado, EstadoEnvio::EmAndamento);
        let motivo = situacao.motivo.unwrap();
        assert!(motivo.starts_with(TEXTO_ATOLADO), "{motivo}");
    }

    #[test]
    fn estado_desconhecido_ou_generico_da_fila_nao_vira_alarme() {
        // Erro genérico, problema com papel, servidor desconhecido, bit sem nome, imprimindo,
        // economia de energia e pausada: nada disso diz que falta papel, atolou ou não responde.
        for fila in [
            0,
            0x2,
            0x40,
            0x0080_0000,
            0x0800_0000,
            0x400,
            0x0100_0000,
            0x1,
        ] {
            let situacao = com_fila(IMPRIMINDO, fila);
            assert_eq!(situacao.estado, EstadoEnvio::EmAndamento, "{fila:#x}");
            assert_eq!(situacao.motivo, None, "{fila:#x}");
        }
    }

    #[test]
    fn fila_que_nao_pode_ser_lida_deixa_so_o_que_o_trabalho_diz() {
        assert_eq!(traduzir_com_fila(IMPRIMINDO, None).motivo, None);
        assert_eq!(
            traduzir_com_fila(COMPLETO, None).estado,
            EstadoEnvio::Concluido
        );
    }

    #[test]
    fn frente_terminada_confirma_mesmo_com_a_bandeja_vazia() {
        for status in [IMPRESSO, COMPLETO, RETIDO, IMPRESSO | EXCLUINDO] {
            for fila in [FILA_SEM_PAPEL, FILA_ATOLADA, FILA_OFFLINE] {
                let situacao = com_fila(status, fila);
                assert_eq!(
                    situacao.estado,
                    EstadoEnvio::Concluido,
                    "{status:#x} {fila:#x}"
                );
                assert_eq!(situacao.motivo, None, "{status:#x} {fila:#x}");
            }
        }
    }

    #[test]
    fn exclusao_continua_falha_mesmo_com_a_fila_em_problema() {
        assert_eq!(
            com_fila(EXCLUINDO, FILA_SEM_PAPEL).estado,
            EstadoEnvio::Falhou
        );
    }

    #[test]
    fn mensagem_do_driver_fica_em_campo_proprio_no_aviso_e_o_motivo_nao_muda() {
        let (motivo, mensagem) = motivo_e_mensagem(
            EstadoEnvio::EmAndamento,
            Some(TEXTO_ATOLADO.to_string()),
            "Bandeja 2",
        );
        assert_eq!(motivo.as_deref(), Some(TEXTO_ATOLADO));
        assert_eq!(mensagem.as_deref(), Some("Bandeja 2"));
    }

    #[test]
    fn sem_motivo_a_mensagem_do_driver_nao_abre_aviso() {
        assert_eq!(
            motivo_e_mensagem(EstadoEnvio::EmAndamento, None, "Imprimindo página 3"),
            (None, None)
        );
    }

    #[test]
    fn na_falha_a_mensagem_do_driver_limpa_segue_junto_do_motivo() {
        let (motivo, mensagem) = motivo_e_mensagem(
            EstadoEnvio::Falhou,
            Some("o trabalho deu erro".to_string()),
            "<b>Tampa</b>\naberta",
        );
        assert_eq!(
            motivo.as_deref(),
            Some("o trabalho deu erro (Tampa aberta)")
        );
        assert_eq!(mensagem, None);
    }

    #[test]
    fn texto_do_driver_perde_marcacao_controle_e_invisiveis() {
        for (entrada, esperado) in [
            ("<b>Bandeja</b> <i>2</i>", "Bandeja 2"),
            ("<script>alert(1)</script>Tampa", "alert(1) Tampa"),
            ("<!-- nota -->Tampa aberta", "Tampa aberta"),
            ("toner < 10% > limite", "toner 10% limite"),
            ("\u{1b}[31mSem toner\u{1b}[0m", "Sem toner"),
            ("\u{9b}1mSem toner", "Sem toner"),
            ("\u{1b}]0;titulo\u{7}Sem toner", "Sem toner"),
            ("linha\r\n  com\tespaços", "linha com espaços"),
            ("a\u{7}b\u{0}c", "abc"),
            ("abc\u{202E}def\u{200B}g\u{FEFF}", "abcdefg"),
            ("Bandeja (2) vazia", "Bandeja (2) vazia"),
            ("Bandeja 2 — sem papel", "Bandeja 2 — sem papel"),
        ] {
            assert_eq!(
                limpar_mensagem_do_driver(entrada).as_deref(),
                Some(esperado),
                "{entrada:?}"
            );
        }
    }

    #[test]
    fn texto_do_driver_sem_nada_que_se_leia_nao_e_mensagem() {
        for entrada in [
            "",
            "   ",
            "\r\n\t",
            "<b></b>",
            "\u{200B}\u{202E}",
            "\u{1b}[0m",
            "<sem fim",
            "\u{1b}[",
            "\u{1b}",
        ] {
            assert_eq!(limpar_mensagem_do_driver(entrada), None, "{entrada:?}");
        }
    }

    #[test]
    fn texto_do_driver_tem_tamanho_limitado_sem_quebrar_caractere() {
        let longa = limpar_mensagem_do_driver(&"é".repeat(100_000)).unwrap();
        assert_eq!(longa.chars().count(), MAXIMO_DA_MENSAGEM_DO_DRIVER);
        assert!(longa.ends_with('…'));
        let exata = "x".repeat(MAXIMO_DA_MENSAGEM_DO_DRIVER);
        assert_eq!(
            limpar_mensagem_do_driver(&exata).as_deref(),
            Some(exata.as_str())
        );
        // Muitos sinais de abertura seguidos não custam mais que uma passada.
        assert_eq!(limpar_mensagem_do_driver(&"<a".repeat(1_000_000)), None);
    }

    #[test]
    fn bits_desconhecidos_nao_encerram_nem_derrubam() {
        assert_eq!(
            traduzir_status(0xFFFF_0000).estado,
            EstadoEnvio::EmAndamento
        );
    }
}
