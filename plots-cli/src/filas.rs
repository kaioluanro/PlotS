//! Comando `filas`: lista as filas do Windows, com estado, e as impressoras simuladas.
//! Só lê o estado; nada é enviado a nenhuma impressora.

use crate::calibrar;
use crate::ImpressoraVirtual;
use plots_perfis::{EntradaPerfil, Perfis};
use plots_windows::{Condicao, Estado, Fila};
use std::io::Write;
use std::path::PathBuf;

/// Primeira linha da listagem: deixa claro que nada foi enviado.
const NOTA_NAO_ENVIA: &str =
    "Filas do Windows. Esta listagem só lê o estado: nada foi enviado a nenhuma impressora.";
/// Aviso sobre a origem do estado.
const NOTA_ESTADO_DRIVER: &str =
    "O estado vem do driver de cada fila e pode não refletir a impressora.";
const MSG_SEM_FILAS: &str = "Nenhuma fila do Windows encontrada.";
const TITULO_SIMULADAS: &str =
    "Impressoras simuladas (não são filas do Windows; nunca enviam nada):";

/// Executa `filas [--perfis ARQUIVO]` e escreve a listagem em `saida`.
pub(crate) fn executar(args: &[String], saida: &mut impl Write) -> Result<(), String> {
    let mut perfis_informado: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        let chave = &args[i];
        let valor = args
            .get(i + 1)
            .ok_or_else(|| format!("faltou valor para {chave}"))?;
        match chave.as_str() {
            "--perfis" if perfis_informado.is_none() => {
                perfis_informado = Some(PathBuf::from(valor));
            }
            _ => return Err(format!("opção desconhecida ou repetida: {chave}")),
        }
        i += 2;
    }
    let caminho = calibrar::caminho_perfis(perfis_informado)?;
    let perfis = calibrar::carregar_perfis(&caminho)?;
    let filas = plots_windows::listar_filas().map_err(|e| e.to_string())?;
    let texto = formatar_listagem(&filas, &perfis);
    write!(saida, "{texto}").map_err(|e| e.to_string())
}

/// Monta o texto da listagem a partir das filas dadas. Não consulta o sistema.
pub(crate) fn formatar_listagem(filas: &[Fila], perfis: &Perfis) -> String {
    let mut texto = String::new();
    texto.push_str(NOTA_NAO_ENVIA);
    texto.push('\n');
    if filas.is_empty() {
        texto.push('\n');
        texto.push_str(MSG_SEM_FILAS);
        texto.push('\n');
    }
    for fila in filas {
        texto.push_str(&bloco_da_fila(fila, perfis));
    }
    texto.push('\n');
    texto.push_str(TITULO_SIMULADAS);
    texto.push('\n');
    for simulada in [ImpressoraVirtual::Traseira, ImpressoraVirtual::Gaveta] {
        texto.push_str("  ");
        texto.push_str(simulada.nome());
        texto.push('\n');
    }
    texto.push('\n');
    texto.push_str(NOTA_ESTADO_DRIVER);
    texto.push('\n');
    texto
}

fn bloco_da_fila(fila: &Fila, perfis: &Perfis) -> String {
    let padrao = if fila.padrao { "sim" } else { "não" };
    let perfil = match perfis.obter(&fila.nome) {
        Some(EntradaPerfil {
            modelo: Some(modelo),
            ..
        }) => format!("sim, modelo {}", crate::nome_do_modelo(*modelo)),
        Some(_) => "sim".to_string(),
        None => format!(
            "não; calibre com plots calibrar \"{0}\" --saida DIRETORIO ou use plots perfil \"{0}\" --modelo jato|laser",
            fila.nome
        ),
    };
    let porta = if fila.porta.is_empty() {
        "(não informada)"
    } else {
        fila.porta.as_str()
    };
    format!(
        "\n{nome}\n  estado: {estado}\n  driver: {driver}\n  porta: {porta}\n  padrão: {padrao}\n  perfil salvo: {perfil}\n",
        nome = fila.nome,
        estado = estado_em_palavras(&fila.estado),
        driver = fila.driver,
    )
}

/// Estado em português: "pronta", ou as condições separadas por vírgula.
pub(crate) fn estado_em_palavras(estado: &Estado) -> String {
    if estado.pronta() {
        return "pronta".into();
    }
    estado
        .condicoes()
        .iter()
        .map(rotulo_da_condicao)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Rótulo em português de uma condição. Desconhecidos mostram o código do bit.
pub(crate) fn rotulo_da_condicao(condicao: &Condicao) -> String {
    let texto = match condicao {
        Condicao::Pausada => "pausada",
        Condicao::Offline => "offline",
        Condicao::SemPapel => "sem papel",
        Condicao::Atolada => "atolada",
        Condicao::Erro => "erro",
        Condicao::ExclusaoPendente => "exclusão pendente",
        Condicao::AlimentacaoManual => "alimentação manual",
        Condicao::ProblemaComPapel => "problema com papel",
        Condicao::RecebendoDados => "recebendo dados",
        Condicao::Ocupada => "ocupada",
        Condicao::Imprimindo => "imprimindo",
        Condicao::BandejaCheia => "bandeja de saída cheia",
        Condicao::Indisponivel => "indisponível",
        Condicao::EmEspera => "em espera",
        Condicao::Processando => "processando",
        Condicao::Inicializando => "inicializando",
        Condicao::Aquecendo => "aquecendo",
        Condicao::PoucoToner => "pouco toner",
        Condicao::SemToner => "sem toner",
        Condicao::PaginaNaoImpressa => "página não impressa",
        Condicao::IntervencaoDoUsuario => "precisa de intervenção do usuário",
        Condicao::SemMemoria => "sem memória",
        Condicao::TampaAberta => "tampa aberta",
        Condicao::ServidorDesconhecido => "servidor desconhecido",
        Condicao::EconomiaDeEnergia => "economia de energia",
        Condicao::ServidorOffline => "servidor offline",
        Condicao::DriverDesatualizado => "driver precisa de atualização",
        Condicao::Outro(bit) => return format!("outro (código {bit:#010X})"),
    };
    texto.to_string()
}

#[cfg(test)]
mod testes {
    use super::*;
    use plots_core::ModeloImpressora;

    fn fila(nome: &str, bits: u32, padrao: bool) -> Fila {
        Fila {
            nome: nome.into(),
            driver: "Driver Exemplo".into(),
            porta: "USB001".into(),
            padrao,
            estado: Estado::de_bits(bits),
        }
    }

    fn com_perfil(nome: &str) -> Perfis {
        let mut perfis = Perfis::new();
        perfis.inserir(nome, EntradaPerfil::default());
        perfis
    }

    #[test]
    fn fila_pronta_mostra_campos_e_perfil_salvo() {
        let filas = [fila("Jato Escritório", 0, true)];
        let texto = formatar_listagem(&filas, &com_perfil("Jato Escritório"));
        assert!(texto.contains("Jato Escritório\n  estado: pronta\n"));
        assert!(texto.contains("  driver: Driver Exemplo\n"));
        assert!(texto.contains("  porta: USB001\n"));
        assert!(texto.contains("  padrão: sim\n"));
        assert!(texto.contains("  perfil salvo: sim\n"));
    }

    #[test]
    fn fila_sem_perfil_sugere_calibrar_com_o_nome_exato() {
        let filas = [fila("Laser Gaveta", 0, false)];
        let texto = formatar_listagem(&filas, &Perfis::new());
        assert!(texto.contains("  padrão: não\n"));
        assert!(texto.contains(
            "  perfil salvo: não; calibre com plots calibrar \"Laser Gaveta\" --saida DIRETORIO ou use plots perfil \"Laser Gaveta\" --modelo jato|laser\n"
        ));
    }

    #[test]
    fn fila_com_modelo_mostra_o_modelo_e_sem_modelo_nao_inventa() {
        let filas = [
            fila("A", 0, false),
            fila("B", 0, false),
            fila("C", 0, false),
        ];
        let mut perfis = com_perfil("C");
        perfis.definir_modelo("A", ModeloImpressora::JatoDeTinta);
        perfis.definir_modelo("B", ModeloImpressora::Laser);
        let texto = formatar_listagem(&filas, &perfis);
        assert!(
            texto.contains("  perfil salvo: sim, modelo Jato de tinta\n"),
            "{texto}"
        );
        assert!(
            texto.contains("  perfil salvo: sim, modelo Laser\n"),
            "{texto}"
        );
        assert!(texto.contains("  perfil salvo: sim\n"), "{texto}");
    }

    #[test]
    fn estados_combinados_aparecem_em_palavras_separadas_por_virgula() {
        let estado = Estado::de_bits(0x0000_0001 | 0x0000_0080);
        assert_eq!(estado_em_palavras(&estado), "pausada, offline");
        let atolada = Estado::de_bits(0x0000_0008 | 0x0000_0010);
        assert_eq!(estado_em_palavras(&atolada), "sem papel, atolada");
    }

    #[test]
    fn bit_sem_traducao_aparece_como_outro_com_codigo() {
        let estado = Estado::de_bits(0x0800_0000);
        assert_eq!(estado_em_palavras(&estado), "outro (código 0x08000000)");
    }

    #[test]
    fn cada_condicao_tem_rotulo_em_portugues() {
        let casos = [
            (Condicao::Pausada, "pausada"),
            (Condicao::Offline, "offline"),
            (Condicao::SemPapel, "sem papel"),
            (Condicao::Atolada, "atolada"),
            (Condicao::Erro, "erro"),
            (Condicao::ExclusaoPendente, "exclusão pendente"),
            (Condicao::AlimentacaoManual, "alimentação manual"),
            (Condicao::ProblemaComPapel, "problema com papel"),
            (Condicao::RecebendoDados, "recebendo dados"),
            (Condicao::Ocupada, "ocupada"),
            (Condicao::Imprimindo, "imprimindo"),
            (Condicao::BandejaCheia, "bandeja de saída cheia"),
            (Condicao::Indisponivel, "indisponível"),
            (Condicao::EmEspera, "em espera"),
            (Condicao::Processando, "processando"),
            (Condicao::Inicializando, "inicializando"),
            (Condicao::Aquecendo, "aquecendo"),
            (Condicao::PoucoToner, "pouco toner"),
            (Condicao::SemToner, "sem toner"),
            (Condicao::PaginaNaoImpressa, "página não impressa"),
            (
                Condicao::IntervencaoDoUsuario,
                "precisa de intervenção do usuário",
            ),
            (Condicao::SemMemoria, "sem memória"),
            (Condicao::TampaAberta, "tampa aberta"),
            (Condicao::ServidorDesconhecido, "servidor desconhecido"),
            (Condicao::EconomiaDeEnergia, "economia de energia"),
            (Condicao::ServidorOffline, "servidor offline"),
            (
                Condicao::DriverDesatualizado,
                "driver precisa de atualização",
            ),
        ];
        for (condicao, esperado) in casos {
            assert_eq!(rotulo_da_condicao(&condicao), esperado);
        }
    }

    #[test]
    fn uso_comum_da_impressora_aparece_em_palavras() {
        let imprimindo = Estado::de_bits(0x0000_0400);
        assert_eq!(estado_em_palavras(&imprimindo), "imprimindo");
        let tampa = Estado::de_bits(0x0040_0000 | 0x0002_0000);
        assert_eq!(estado_em_palavras(&tampa), "pouco toner, tampa aberta");
    }

    #[test]
    fn atributo_usar_offline_aparece_como_offline() {
        let estado = plots_windows::Estado::de_status_e_atributos(0, 0x0000_0400);
        assert_eq!(estado_em_palavras(&estado), "offline");
    }

    #[test]
    fn nome_com_acento_e_espaco_sai_exato() {
        let nome = "Impressora Lançamento — Escritório";
        let filas = [fila(nome, 0, false)];
        let texto = formatar_listagem(&filas, &com_perfil(nome));
        assert!(texto.contains(&format!("\n{nome}\n  estado: pronta\n")));
        assert!(texto.contains("  perfil salvo: sim\n"));
    }

    #[test]
    fn lista_vazia_nao_e_erro_e_mostra_as_simuladas() {
        let texto = formatar_listagem(&[], &Perfis::new());
        assert!(texto.contains(MSG_SEM_FILAS));
        assert!(texto.contains(ImpressoraVirtual::Traseira.nome()));
        assert!(texto.contains(ImpressoraVirtual::Gaveta.nome()));
        assert!(texto.contains("não são filas do Windows"));
    }

    #[test]
    fn saida_declara_que_nada_foi_enviado_e_que_o_estado_e_do_driver() {
        let texto = formatar_listagem(&[], &Perfis::new());
        assert!(texto.contains("nada foi enviado a nenhuma impressora"));
        assert!(texto.contains("O estado vem do driver"));
    }

    #[test]
    fn porta_vazia_mostra_texto_de_ausencia() {
        let mut sem_porta = fila("Sem Porta", 0, false);
        sem_porta.porta.clear();
        let texto = formatar_listagem(&[sem_porta], &Perfis::new());
        assert!(texto.contains("  porta: (não informada)\n"));
    }

    #[test]
    fn opcao_desconhecida_ou_sem_valor_falha_antes_de_listar() {
        let mut saida = Vec::new();
        let sem_valor = vec!["filas".to_string(), "--perfis".to_string()];
        assert_eq!(
            executar(&sem_valor, &mut saida).unwrap_err(),
            "faltou valor para --perfis"
        );
        let desconhecida = vec![
            "filas".to_string(),
            "--imprimir".to_string(),
            "x".to_string(),
        ];
        assert!(executar(&desconhecida, &mut saida)
            .unwrap_err()
            .starts_with("opção desconhecida"));
        assert!(saida.is_empty());
    }
}
