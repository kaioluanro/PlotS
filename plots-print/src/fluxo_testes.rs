use super::*;
use plots_core::{plano_lotes_por_papel, DimensoesPagina, OrdemPassadas, PerfilImpressora};
use std::collections::VecDeque;

const A4: DimensoesPagina = DimensoesPagina::new(210.0, 297.0);
const A5: DimensoesPagina = DimensoesPagina::new(148.0, 210.0);
const SEM_DEMORA: Espera = Espera {
    intervalo: Duration::ZERO,
    consultas: 10,
};

#[derive(Clone, Debug, PartialEq)]
enum Registro {
    Enviou(Passada, Vec<Option<usize>>),
    Cancelou,
}

#[derive(Clone, Copy)]
enum Leitura {
    Estado(EstadoEnvio),
    Aviso(&'static str),
    Erro,
}

/// Backend que só registra o que recebeu: nenhum teste daqui alcança uma fila.
#[derive(Default)]
struct Falso {
    registro: Vec<Registro>,
    leituras: VecDeque<Leitura>,
    falha_ao_enviar: bool,
    motivo: Option<String>,
    saiu: bool,
}

impl Backend for Falso {
    type Erro = String;
    fn enviar(&mut self, passada: Passada, paginas: &[Pagina]) -> Result<(), String> {
        if self.falha_ao_enviar {
            return Err("recusado".to_string());
        }
        self.registro.push(Registro::Enviou(
            passada,
            paginas.iter().map(|p| p.numero).collect(),
        ));
        Ok(())
    }
    fn consultar(&mut self) -> Result<EstadoEnvio, String> {
        self.motivo = None;
        match self
            .leituras
            .pop_front()
            .unwrap_or(Leitura::Estado(EstadoEnvio::Concluido))
        {
            Leitura::Estado(EstadoEnvio::Falhou) => {
                self.motivo = Some("deu erro".to_string());
                Ok(EstadoEnvio::Falhou)
            }
            Leitura::Estado(estado) => Ok(estado),
            Leitura::Aviso(motivo) => {
                self.motivo = Some(motivo.to_string());
                Ok(EstadoEnvio::EmAndamento)
            }
            Leitura::Erro => Err("sem resposta".to_string()),
        }
    }
    fn cancelar(&mut self) -> Result<(), String> {
        self.registro.push(Registro::Cancelou);
        Ok(())
    }
}

impl DescreveFalha for Falso {
    fn motivo(&self) -> Option<String> {
        self.motivo.clone()
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        self.saiu
    }
}

fn lotes(
    dimensoes: &[DimensoesPagina],
    folhas: usize,
    perfil: PerfilImpressora,
) -> Vec<LoteDoFluxo> {
    lotes_do_plano(
        &plano_lotes_por_papel(dimensoes, folhas, OrdemPassadas::default(), perfil).unwrap(),
    )
}

fn com_limite(leituras: &[Leitura], consultas: u32) -> Condutor<Falso> {
    let backend = Falso {
        leituras: leituras.iter().copied().collect(),
        ..Falso::default()
    };
    Condutor::novo(
        backend,
        lotes(&[A4; 4], 1, PerfilImpressora::default()),
        Espera {
            intervalo: Duration::ZERO,
            consultas,
        },
    )
    .unwrap()
}

fn condutor(leituras: &[Leitura]) -> Condutor<Falso> {
    com_limite(leituras, SEM_DEMORA.consultas)
}

#[test]
fn frente_e_verso_de_cada_lote_terminam_o_documento() {
    let mut fluxo = condutor(&[]);
    assert_eq!((fluxo.numero(), fluxo.total()), (1, 2));
    fluxo.enviar_frente().unwrap();
    assert!(matches!(fluxo.consultar(), Passo::FrenteConfirmada));
    fluxo.enviar_verso().unwrap();
    assert!(matches!(fluxo.consultar(), Passo::LoteConcluido));
    assert_eq!(fluxo.numero(), 2);
    fluxo.enviar_frente().unwrap();
    assert!(matches!(fluxo.consultar(), Passo::FrenteConfirmada));
    fluxo.enviar_verso().unwrap();
    assert!(matches!(fluxo.consultar(), Passo::DocumentoConcluido));
    assert_eq!(fluxo.numero(), 2);
    assert_eq!(fluxo.estado(), EstadoJob::Concluido);
    assert!(fluxo.lote().is_some());
}

#[test]
fn verso_nao_sai_antes_de_a_fila_confirmar_a_frente() {
    let mut fluxo = condutor(&[Leitura::Estado(EstadoEnvio::EmAndamento)]);
    fluxo.enviar_frente().unwrap();
    assert!(fluxo.enviar_verso().is_err());
    assert!(matches!(fluxo.consultar(), Passo::EmAndamento));
    assert!(fluxo.enviar_verso().is_err());
    assert!(matches!(fluxo.consultar(), Passo::FrenteConfirmada));
    fluxo.enviar_verso().unwrap();
    let versos = fluxo
        .backend()
        .registro
        .iter()
        .filter(|item| matches!(item, Registro::Enviou(Passada::Verso, _)))
        .count();
    assert_eq!(versos, 1);
}

#[test]
fn aviso_so_e_novo_uma_vez_por_motivo_e_nao_gasta_o_limite() {
    let mut fluxo = com_limite(
        &[
            Leitura::Aviso("sem papel"),
            Leitura::Aviso("sem papel"),
            Leitura::Aviso("offline"),
            Leitura::Aviso("offline"),
        ],
        1,
    );
    fluxo.enviar_frente().unwrap();
    let novos: Vec<bool> = (0..4)
        .map(|_| match fluxo.consultar() {
            Passo::Aviso { novo, .. } => novo,
            outro => panic!("{outro:?}"),
        })
        .collect();
    assert_eq!(novos, [true, false, true, false]);
    assert!(matches!(fluxo.consultar(), Passo::FrenteConfirmada));
}

#[test]
fn limite_de_consultas_para_sem_cancelar_e_continuar_rearma() {
    let mut fluxo = com_limite(&[Leitura::Estado(EstadoEnvio::EmAndamento)], 1);
    fluxo.enviar_frente().unwrap();
    assert!(matches!(fluxo.consultar(), Passo::EmAndamento));
    assert!(matches!(fluxo.consultar(), Passo::TempoEsgotado));
    assert!(!fluxo.backend().registro.contains(&Registro::Cancelou));
    fluxo.continuar_esperando();
    assert!(matches!(fluxo.consultar(), Passo::FrenteConfirmada));
}

#[test]
fn erros_seguidos_de_consulta_param_sem_apagar_o_trabalho() {
    let leituras = [Leitura::Erro; ERROS_DE_CONSULTA_SEGUIDOS as usize];
    let mut fluxo = com_limite(&leituras, 100);
    fluxo.enviar_frente().unwrap();
    for _ in 1..ERROS_DE_CONSULTA_SEGUIDOS {
        assert!(matches!(fluxo.consultar(), Passo::EmAndamento));
    }
    assert!(matches!(
        fluxo.consultar(),
        Passo::SemAcompanhamento {
            saiu_da_fila: false,
            ..
        }
    ));
    assert!(!fluxo.backend().registro.contains(&Registro::Cancelou));
}

#[test]
fn cancelar_tira_da_fila_e_volta_ao_mesmo_lote_sem_enviar_de_novo() {
    let mut fluxo = condutor(&[Leitura::Estado(EstadoEnvio::EmAndamento)]);
    fluxo.enviar_frente().unwrap();
    assert!(matches!(fluxo.consultar(), Passo::EmAndamento));
    fluxo.cancelar().unwrap();
    assert_eq!(fluxo.estado(), EstadoJob::Pronto);
    assert_eq!(fluxo.numero(), 1);
    let enviados = fluxo
        .backend()
        .registro
        .iter()
        .filter(|item| matches!(item, Registro::Enviou(..)))
        .count();
    assert_eq!(enviados, 1);
    assert_eq!(fluxo.backend().registro.last(), Some(&Registro::Cancelou));
}

#[test]
fn reimprimir_depois_de_falha_refaz_so_o_lote_atual() {
    let mut fluxo = condutor(&[
        Leitura::Estado(EstadoEnvio::Concluido),
        Leitura::Estado(EstadoEnvio::Concluido),
        Leitura::Estado(EstadoEnvio::Concluido),
        Leitura::Estado(EstadoEnvio::Falhou),
    ]);
    fluxo.enviar_frente().unwrap();
    fluxo.consultar();
    fluxo.enviar_verso().unwrap();
    fluxo.consultar();
    fluxo.enviar_frente().unwrap();
    fluxo.consultar();
    fluxo.enviar_verso().unwrap();
    assert!(matches!(
        fluxo.consultar(),
        Passo::Falhou { motivo: Some(_) }
    ));
    assert_eq!(fluxo.estado(), EstadoJob::Erro);
    assert_eq!(fluxo.numero(), 2);
    fluxo.reimprimir_lote().unwrap();
    assert_eq!(fluxo.estado(), EstadoJob::FrenteEnviada);
    assert_eq!(fluxo.numero(), 2);
    let registro = &fluxo.backend().registro;
    assert_eq!(registro.len(), 6);
    assert_eq!(registro[4], Registro::Cancelou);
    let (Registro::Enviou(Passada::Frente, de_novo), Registro::Enviou(Passada::Frente, antes)) =
        (&registro[5], &registro[2])
    else {
        panic!("{registro:?}");
    };
    assert_eq!(de_novo, antes);
}

#[test]
fn falha_ao_enviar_fica_no_mesmo_lote_e_a_nova_tentativa_tambem_devolve_erro() {
    let mut fluxo = Condutor::novo(
        Falso {
            falha_ao_enviar: true,
            ..Falso::default()
        },
        lotes(&[A4; 2], 1, PerfilImpressora::default()),
        SEM_DEMORA,
    )
    .unwrap();
    assert!(fluxo.enviar_frente().is_err());
    assert_eq!(fluxo.estado(), EstadoJob::Erro);
    assert_eq!(fluxo.numero(), 1);
    assert!(fluxo.reimprimir_lote().is_err());
    assert_eq!(fluxo.estado(), EstadoJob::Erro);
    assert_eq!(fluxo.backend().registro, [Registro::Cancelou]);
}

#[test]
fn duas_impressoras_avancam_sem_uma_depender_da_outra() {
    let mut uma = condutor(&[]);
    let mut outra = condutor(&[Leitura::Estado(EstadoEnvio::EmAndamento)]);
    uma.enviar_frente().unwrap();
    outra.enviar_frente().unwrap();
    assert!(matches!(uma.consultar(), Passo::FrenteConfirmada));
    assert!(matches!(outra.consultar(), Passo::EmAndamento));
    uma.enviar_verso().unwrap();
    assert!(matches!(uma.consultar(), Passo::LoteConcluido));
    assert_eq!(uma.numero(), 2);
    assert_eq!(outra.numero(), 1);
    assert!(outra.enviar_verso().is_err());
    assert!(matches!(outra.consultar(), Passo::FrenteConfirmada));
    outra.enviar_verso().unwrap();
    assert_eq!(uma.estado(), EstadoJob::Pronto);
    assert_eq!(outra.estado(), EstadoJob::VersoEnviado);
}

#[test]
fn troca_de_papel_so_aparece_quando_a_folha_muda() {
    let mut fluxo = Condutor::novo(
        Falso::default(),
        lotes(&[A4, A4, A5, A5, A5, A5], 2, PerfilImpressora::default()),
        SEM_DEMORA,
    )
    .unwrap();
    let mut trocas = Vec::new();
    for _ in 0..fluxo.total() {
        trocas.push(fluxo.troca_de_papel().map(|papel| papel.formato));
        fluxo.enviar_frente().unwrap();
        fluxo.consultar();
        fluxo.enviar_verso().unwrap();
        fluxo.consultar();
    }
    assert_eq!(trocas, [None, Some(FormatoPapel::A5)]);
}

fn numeros(paginas: &[Pagina]) -> Vec<Option<usize>> {
    paginas.iter().map(|p| p.numero).collect()
}

fn rodar_simulado(modelo: ModeloImpressora, paginas: usize) -> (usize, usize, Lote) {
    let plano = lotes(&vec![A4; paginas], 50, modelo.perfil());
    let lote = plano[0].lote.clone();
    let mut fluxo = Condutor::novo(
        SimGuiado::novo(Mecanica::do_modelo(modelo), modelo.gesto().into()),
        plano,
        SEM_DEMORA,
    )
    .unwrap();
    fluxo.enviar_frente().unwrap();
    assert!(matches!(fluxo.consultar(), Passo::FrenteConfirmada));
    fluxo.enviar_verso().unwrap();
    assert!(matches!(fluxo.consultar(), Passo::DocumentoConcluido));
    let (corretas, folhas) = fluxo.backend().folhas_conferidas(&lote.paginas).unwrap();
    (corretas, folhas, lote)
}

#[test]
fn simulacao_do_jato_segue_a_sequencia_do_modelo_e_confere_as_folhas() {
    let (corretas, folhas, lote) = rodar_simulado(ModeloImpressora::JatoDeTinta, 6);
    assert_eq!((corretas, folhas), (3, 3));
    assert_eq!(numeros(&lote.passada1), [Some(6), Some(4), Some(2)]);
    assert_eq!(numeros(&lote.passada2), [Some(5), Some(3), Some(1)]);
    assert!(lote.passada2.iter().all(|p| p.girar_180));
}

#[test]
fn simulacao_da_laser_segue_a_sequencia_do_modelo_e_confere_as_folhas() {
    let (corretas, folhas, lote) = rodar_simulado(ModeloImpressora::Laser, 6);
    assert_eq!((corretas, folhas), (3, 3));
    assert_eq!(numeros(&lote.passada1), [Some(6), Some(4), Some(2)]);
    assert_eq!(numeros(&lote.passada2), [Some(1), Some(3), Some(5)]);
    assert!(lote.passada2.iter().all(|p| !p.girar_180));
}

#[test]
fn instrucao_de_cada_modelo_bate_com_as_frases_do_terminal() {
    assert_eq!(
        Instrucao::do_modelo(ModeloImpressora::JatoDeTinta),
        Instrucao {
            lado: Lado::Baixo,
            virada: Virada::PelaBordaDeCima
        }
    );
    assert_eq!(
        Instrucao::do_modelo(ModeloImpressora::Laser),
        Instrucao {
            lado: Lado::Baixo,
            virada: Virada::NaoVirar
        }
    );
    let (gesto, colocacao) = frases_de_recolocacao_do_modelo(ModeloImpressora::Laser);
    assert_eq!(gesto, "Não vire o maço.");
    assert!(colocacao.ends_with("com a face impressa para baixo."));
    assert!(vira_para_ler(ModeloImpressora::Laser));
    assert!(!vira_para_ler(ModeloImpressora::JatoDeTinta));
}

#[test]
fn papel_fora_da_tabela_tem_nome_pelas_medidas() {
    let plano = lotes(
        &[DimensoesPagina::new(100.0, 100.0)],
        1,
        PerfilImpressora::default(),
    );
    assert_eq!(nome_do_papel(&plano[0].papel), "100 × 100 mm");
}

#[test]
fn documento_sem_lotes_e_recusado_sem_travar() {
    assert!(Condutor::novo(Falso::default(), Vec::new(), SEM_DEMORA).is_err());
}

#[test]
fn gesto_vem_antes_da_colocacao_em_toda_combinacao() {
    let mecanicas = [
        Mecanica::do_modelo(ModeloImpressora::JatoDeTinta),
        Mecanica::do_modelo(ModeloImpressora::Laser),
    ];
    let gestos = [
        (
            true,
            true,
            "Vire o maço pela borda de cima, como um bloco de notas.",
        ),
        (
            true,
            false,
            "Vire o maço pela lateral, como a página de um livro.",
        ),
        (false, true, "Gire o maço 180°, sem virá-lo."),
        (false, false, "Não vire o maço."),
    ];
    for mecanica in mecanicas {
        for (virar_maco, girar_180, texto) in gestos {
            let gesto = Recolocacao {
                virar_maco,
                girar_180,
                ..Recolocacao::default()
            };
            assert_eq!(texto_do_gesto(gesto), texto);
            let [primeira, segunda] = linhas_de_recolocacao(mecanica, gesto);
            assert_eq!(primeira, texto, "o gesto abre a recolocacao");
            assert!(
                segunda.starts_with("Depois, sem mudar a ordem das folhas, coloque na entrada com a face impressa para "),
                "{segunda}"
            );
            assert!(
                segunda.ends_with("cima.") || segunda.ends_with("baixo."),
                "{segunda}"
            );
        }
    }
}

#[test]
fn gesto_de_cada_modelo_vem_antes_da_colocacao() {
    for modelo in [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser] {
        let (gesto, colocacao) = frases_de_recolocacao_do_modelo(modelo);
        assert!(!gesto.is_empty());
        assert!(
            colocacao.starts_with("Depois, sem mudar a ordem das folhas, coloque na entrada"),
            "{colocacao}"
        );
    }
}
