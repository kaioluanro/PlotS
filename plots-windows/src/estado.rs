//! Tradução dos bits de estado de uma fila do Windows em condições legíveis.
//!
//! Função pura: não consulta impressora nem o sistema. Os bits vêm do campo de
//! estado da fila e o que cada driver reporta depende dele; por isso um bit
//! desconhecido não é descartado e segue no resultado como `Condicao::Outro`.

/// Bits de `PRINTER_STATUS_*` com condição própria (valores da documentação do winspool).
const PAUSADA: u32 = 0x0000_0001;
const ERRO: u32 = 0x0000_0002;
const EXCLUSAO_PENDENTE: u32 = 0x0000_0004;
const ATOLADA: u32 = 0x0000_0008;
const SEM_PAPEL: u32 = 0x0000_0010;
const ALIMENTACAO_MANUAL: u32 = 0x0000_0020;
const PROBLEMA_COM_PAPEL: u32 = 0x0000_0040;
const OFFLINE: u32 = 0x0000_0080;
const RECEBENDO_DADOS: u32 = 0x0000_0100;
const OCUPADA: u32 = 0x0000_0200;
const IMPRIMINDO: u32 = 0x0000_0400;
const BANDEJA_CHEIA: u32 = 0x0000_0800;
const INDISPONIVEL: u32 = 0x0000_1000;
const EM_ESPERA: u32 = 0x0000_2000;
const PROCESSANDO: u32 = 0x0000_4000;
const INICIALIZANDO: u32 = 0x0000_8000;
const AQUECENDO: u32 = 0x0001_0000;
const POUCO_TONER: u32 = 0x0002_0000;
const SEM_TONER: u32 = 0x0004_0000;
const PAGINA_NAO_IMPRESSA: u32 = 0x0008_0000;
const INTERVENCAO_DO_USUARIO: u32 = 0x0010_0000;
const SEM_MEMORIA: u32 = 0x0020_0000;
const TAMPA_ABERTA: u32 = 0x0040_0000;
const SERVIDOR_DESCONHECIDO: u32 = 0x0080_0000;
const ECONOMIA_DE_ENERGIA: u32 = 0x0100_0000;
const SERVIDOR_OFFLINE: u32 = 0x0200_0000;
const DRIVER_DESATUALIZADO: u32 = 0x0400_0000;

/// Atributo `PRINTER_ATTRIBUTE_WORK_OFFLINE`: "usar impressora offline", ligado pelo usuário.
const ATRIBUTO_USAR_OFFLINE: u32 = 0x0000_0400;

/// Ordem fixa em que as condições aparecem em `Estado::condicoes`.
const CONHECIDAS: [(u32, Condicao); 27] = [
    (PAUSADA, Condicao::Pausada),
    (OFFLINE, Condicao::Offline),
    (SEM_PAPEL, Condicao::SemPapel),
    (ATOLADA, Condicao::Atolada),
    (ERRO, Condicao::Erro),
    (EXCLUSAO_PENDENTE, Condicao::ExclusaoPendente),
    (ALIMENTACAO_MANUAL, Condicao::AlimentacaoManual),
    (PROBLEMA_COM_PAPEL, Condicao::ProblemaComPapel),
    (RECEBENDO_DADOS, Condicao::RecebendoDados),
    (OCUPADA, Condicao::Ocupada),
    (IMPRIMINDO, Condicao::Imprimindo),
    (BANDEJA_CHEIA, Condicao::BandejaCheia),
    (INDISPONIVEL, Condicao::Indisponivel),
    (EM_ESPERA, Condicao::EmEspera),
    (PROCESSANDO, Condicao::Processando),
    (INICIALIZANDO, Condicao::Inicializando),
    (AQUECENDO, Condicao::Aquecendo),
    (POUCO_TONER, Condicao::PoucoToner),
    (SEM_TONER, Condicao::SemToner),
    (PAGINA_NAO_IMPRESSA, Condicao::PaginaNaoImpressa),
    (INTERVENCAO_DO_USUARIO, Condicao::IntervencaoDoUsuario),
    (SEM_MEMORIA, Condicao::SemMemoria),
    (TAMPA_ABERTA, Condicao::TampaAberta),
    (SERVIDOR_DESCONHECIDO, Condicao::ServidorDesconhecido),
    (ECONOMIA_DE_ENERGIA, Condicao::EconomiaDeEnergia),
    (SERVIDOR_OFFLINE, Condicao::ServidorOffline),
    (DRIVER_DESATUALIZADO, Condicao::DriverDesatualizado),
];

/// Uma condição informada pelo estado da fila.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Condicao {
    Pausada,
    Offline,
    SemPapel,
    Atolada,
    Erro,
    ExclusaoPendente,
    AlimentacaoManual,
    ProblemaComPapel,
    RecebendoDados,
    Ocupada,
    Imprimindo,
    BandejaCheia,
    Indisponivel,
    EmEspera,
    Processando,
    Inicializando,
    Aquecendo,
    PoucoToner,
    SemToner,
    PaginaNaoImpressa,
    IntervencaoDoUsuario,
    SemMemoria,
    TampaAberta,
    ServidorDesconhecido,
    EconomiaDeEnergia,
    ServidorOffline,
    DriverDesatualizado,
    /// Um bit sem tradução, guardado com a máscara exata (um único bit).
    Outro(u32),
}

/// Estado completo de uma fila: os bits recebidos e as condições deles derivadas.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Estado {
    bits: u32,
    condicoes: Vec<Condicao>,
}

impl Estado {
    /// Traduz só a palavra de estado. Nenhum bit é perdido: os desconhecidos viram `Outro`.
    pub fn de_bits(bits: u32) -> Self {
        Self::de_status_e_atributos(bits, 0)
    }

    /// Traduz a palavra de estado e a de atributos. O atributo "usar offline" conta como
    /// offline, mesmo com o bit de offline ausente da palavra de estado.
    ///
    /// Em `bits`, o bit de offline é acrescido quando o atributo está ligado: assim
    /// `pronta()` continua valendo só quando nada está ligado.
    pub fn de_status_e_atributos(status: u32, atributos: u32) -> Self {
        let bits = if atributos & ATRIBUTO_USAR_OFFLINE != 0 {
            status | OFFLINE
        } else {
            status
        };
        let mut condicoes = Vec::new();
        for (mascara, condicao) in CONHECIDAS {
            if bits & mascara != 0 {
                condicoes.push(condicao);
            }
        }
        let conhecidos = CONHECIDAS
            .iter()
            .fold(0u32, |acumulado, (mascara, _)| acumulado | mascara);
        let mut restantes = bits & !conhecidos;
        while restantes != 0 {
            let bit = restantes & restantes.wrapping_neg();
            condicoes.push(Condicao::Outro(bit));
            restantes &= restantes - 1;
        }
        Self { bits, condicoes }
    }

    /// Palavra de estado, com o bit de offline acrescido quando o atributo está ligado.
    pub fn bits(&self) -> u32 {
        self.bits
    }

    /// Condições em ordem fixa; vazio quando a fila está pronta.
    pub fn condicoes(&self) -> &[Condicao] {
        &self.condicoes
    }

    /// `true` quando nenhum bit de estado está ligado.
    pub fn pronta(&self) -> bool {
        self.bits == 0
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Um bit de `PRINTER_STATUS_*` que não tem condição própria (bit 27).
    const BIT_SEM_CONDICAO: u32 = 0x0800_0000;

    #[test]
    fn zero_significa_pronta_sem_condicoes() {
        let estado = Estado::de_bits(0);
        assert!(estado.pronta());
        assert!(estado.condicoes().is_empty());
        assert_eq!(estado.bits(), 0);
    }

    #[test]
    fn cada_bit_conhecido_isolado_vira_a_sua_condicao() {
        for (bit, esperada) in CONHECIDAS {
            let estado = Estado::de_bits(bit);
            assert!(
                !estado.pronta(),
                "bit {bit:#x} deveria deixar a fila não pronta"
            );
            assert_eq!(estado.condicoes(), &[esperada], "bit {bit:#x}");
        }
    }

    #[test]
    fn bits_combinados_mantem_a_ordem_fixa() {
        let estado = Estado::de_bits(ERRO | PAUSADA | OFFLINE);
        assert_eq!(
            estado.condicoes(),
            &[Condicao::Pausada, Condicao::Offline, Condicao::Erro]
        );
        assert_eq!(estado.bits(), ERRO | PAUSADA | OFFLINE);
    }

    #[test]
    fn papel_atolado_e_sem_papel_juntos_aparecem_os_dois() {
        let estado = Estado::de_bits(ATOLADA | SEM_PAPEL);
        assert_eq!(estado.condicoes(), &[Condicao::SemPapel, Condicao::Atolada]);
    }

    #[test]
    fn bit_desconhecido_isolado_vira_outro_com_a_mascara() {
        let estado = Estado::de_bits(BIT_SEM_CONDICAO);
        assert!(!estado.pronta());
        assert_eq!(estado.condicoes(), &[Condicao::Outro(BIT_SEM_CONDICAO)]);
    }

    #[test]
    fn bits_desconhecidos_e_conhecidos_nao_se_perdem() {
        let bits = PROBLEMA_COM_PAPEL | BIT_SEM_CONDICAO | OFFLINE;
        let estado = Estado::de_bits(bits);
        assert_eq!(estado.bits(), bits);
        assert_eq!(
            estado.condicoes(),
            &[
                Condicao::Offline,
                Condicao::ProblemaComPapel,
                Condicao::Outro(BIT_SEM_CONDICAO),
            ]
        );
    }

    #[test]
    fn todos_os_bits_ficam_representados() {
        let estado = Estado::de_bits(u32::MAX);
        let conhecidos = CONHECIDAS.len();
        let outros = estado
            .condicoes()
            .iter()
            .filter(|c| matches!(c, Condicao::Outro(_)))
            .count();
        assert_eq!(estado.condicoes().len(), conhecidos + outros);
        assert_eq!(outros, 32 - conhecidos);
        let soma: u32 = estado
            .condicoes()
            .iter()
            .filter_map(|c| match c {
                Condicao::Outro(bit) => Some(*bit),
                _ => None,
            })
            .sum();
        let mascara_conhecidos = CONHECIDAS.iter().fold(0u32, |a, (m, _)| a | m);
        assert_eq!(soma, u32::MAX & !mascara_conhecidos);
    }

    #[test]
    fn atributo_usar_offline_conta_como_offline() {
        let estado = Estado::de_status_e_atributos(0, ATRIBUTO_USAR_OFFLINE);
        assert!(!estado.pronta());
        assert_eq!(estado.condicoes(), &[Condicao::Offline]);
        assert_eq!(estado.bits(), OFFLINE);
    }

    #[test]
    fn atributo_usar_offline_nao_duplica_o_offline_do_status() {
        let estado = Estado::de_status_e_atributos(OFFLINE, ATRIBUTO_USAR_OFFLINE);
        assert_eq!(estado.condicoes(), &[Condicao::Offline]);
    }

    #[test]
    fn outros_atributos_nao_mudam_o_estado() {
        let estado = Estado::de_status_e_atributos(0, 0x0000_0004);
        assert!(estado.pronta());
    }

    #[test]
    fn bits_de_uso_normal_tem_condicao_propria_e_nao_outro() {
        let uso = [
            IMPRIMINDO,
            OCUPADA,
            AQUECENDO,
            EM_ESPERA,
            ECONOMIA_DE_ENERGIA,
            PROCESSANDO,
            SEM_TONER,
            POUCO_TONER,
            TAMPA_ABERTA,
        ];
        for bits in uso {
            let estado = Estado::de_bits(bits);
            assert!(
                estado
                    .condicoes()
                    .iter()
                    .all(|c| !matches!(c, Condicao::Outro(_))),
                "bit {bits:#x} não deveria virar Outro"
            );
        }
    }
}
