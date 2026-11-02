//! Tradução do estado de um trabalho no spooler, sem consultar o sistema.
//!
//! Limite conhecido: a confirmação do spooler só garante a entrega ao destino seguinte, não a
//! saída do papel. Um trabalho que some da fila depois de enviado também é tratado como
//! terminado, porque o sistema não guarda o resultado de um trabalho que já saiu.

use plots_print::job::EstadoEnvio;

/// Bits de `JOB_STATUS_*` da documentação do winspool.
const ERRO: u32 = 0x0000_0002;
const EXCLUINDO: u32 = 0x0000_0004;
const OFFLINE: u32 = 0x0000_0020;
const SEM_PAPEL: u32 = 0x0000_0040;
const IMPRESSO: u32 = 0x0000_0080;
const EXCLUIDO: u32 = 0x0000_0100;
const BLOQUEADO: u32 = 0x0000_0200;
const INTERVENCAO: u32 = 0x0000_0400;
const COMPLETO: u32 = 0x0000_1000;
const RETIDO: u32 = 0x0000_2000;

/// Bits de quem só espera alguém resolver algo na impressora, com o texto de cada um.
const AVISOS: [(u32, &str); 3] = [
    (SEM_PAPEL, "a impressora está sem papel"),
    (OFFLINE, "a impressora está offline"),
    (INTERVENCAO, "a impressora pede atenção"),
];

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
}

fn textos_ligados(lista: &[(u32, &'static str)], status: u32) -> Vec<&'static str> {
    lista
        .iter()
        .filter(|(bit, _)| status & bit != 0)
        .map(|(_, texto)| *texto)
        .collect()
}

/// Traduz o campo `Status` de `JOB_INFO_2W`.
///
/// Sem papel, offline e pedido de atenção vêm primeiro: o trabalho pode voltar a andar sozinho
/// quando alguém resolver, e por isso segue em andamento, com o motivo, mesmo com o bit de erro
/// que muitos drivers ligam junto. Depois vem a falha, que vence os demais bits porque um
/// trabalho com erro pode continuar marcado como imprimindo.
pub fn traduzir_status(status: u32) -> SituacaoTrabalho {
    let falhas = textos_ligados(&FALHAS, status);
    let avisos = textos_ligados(&AVISOS, status);
    if !avisos.is_empty() {
        let motivos = [avisos, falhas].concat();
        return SituacaoTrabalho {
            estado: EstadoEnvio::EmAndamento,
            motivo: Some(motivos.join("; ")),
        };
    }
    if !falhas.is_empty() {
        return falhou(falhas.join("; "));
    }
    if status & (IMPRESSO | COMPLETO | RETIDO) != 0 {
        return SituacaoTrabalho {
            estado: EstadoEnvio::Concluido,
            motivo: None,
        };
    }
    // Excluir sem ter terminado quer dizer que alguém tirou o trabalho da fila.
    if status & (EXCLUINDO | EXCLUIDO) != 0 {
        return falhou("o trabalho foi excluído da fila antes de terminar".to_string());
    }
    // Os demais bits, inclusive pausado, só pedem para esperar: quem pausou pode retomar.
    SituacaoTrabalho {
        estado: EstadoEnvio::EmAndamento,
        motivo: None,
    }
}

fn falhou(motivo: String) -> SituacaoTrabalho {
    SituacaoTrabalho {
        estado: EstadoEnvio::Falhou,
        motivo: Some(motivo),
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
            (OFFLINE, "offline"),
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
            duas.contains("sem papel") && duas.contains("offline"),
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
    fn bits_desconhecidos_nao_encerram_nem_derrubam() {
        assert_eq!(
            traduzir_status(0xFFFF_0000).estado,
            EstadoEnvio::EmAndamento
        );
    }
}
