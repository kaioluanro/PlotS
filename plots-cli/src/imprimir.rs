//! Comando `imprimir`: grava as passadas e conduz a impressão simulada, lote a lote.

use crate::calibrar::{caminho_perfis, carregar_perfis};
use crate::geracao::{self, Documento, Opcoes};
use crate::{listar, relatar_folhas, ImpressoraVirtual};
use plots_core::{plano_documento, Lote, OrdemPassadas, Pagina, PerfilImpressora};
use plots_print::job::{EstadoJob, Job, SimBackend};
use plots_print::{conferir_folhas, Mecanica, Recolocacao, Simulador, Veredito};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// Documento de referência usado para escolher a mecânica simulada de um perfil salvo.
const REFERENCIA_PAGINAS: usize = 8;
const REFERENCIA_FOLHAS_POR_LOTE: usize = 2;

const MSG_SEM_SIMULAR: &str = crate::MSG_ENVIO_INDISPONIVEL;

/// Fila de destino: a mecânica usada na simulação e o perfil que ordena as passadas.
struct Fila {
    nome: String,
    perfil: PerfilImpressora,
    mecanica: Mecanica,
    /// `true` quando a mecânica foi escolhida para ser coerente com um perfil salvo.
    de_perfil_salvo: bool,
    /// Texto do gesto guardado no perfil salvo, mostrado na linha seguinte à frase.
    gesto_texto: Option<String>,
}

/// Executa `imprimir ARQUIVO --impressora NOME --lote K --saida DIRETORIO --simular ...`.
pub(crate) fn executar(
    args: &[String],
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> Result<(), String> {
    let arquivo = args
        .get(1)
        .filter(|s| !s.starts_with("--"))
        .ok_or("informe o arquivo PDF")?;
    let (mut impressora, mut tamanho, mut diretorio, mut ordem, mut perfis) =
        (None, None, None, None, None);
    let (mut folhas_finais, mut simular) = (false, false);
    let mut i = 2;
    while i < args.len() {
        let chave = &args[i];
        if chave == "--simular" || chave == "--folhas-finais" {
            let marca = if chave == "--simular" {
                &mut simular
            } else {
                &mut folhas_finais
            };
            if *marca {
                return Err(format!("{chave} foi informado mais de uma vez"));
            }
            *marca = true;
            i += 1;
            continue;
        }
        let valor = args
            .get(i + 1)
            .ok_or_else(|| format!("faltou valor para {chave}"))?;
        match chave.as_str() {
            "--impressora" if impressora.is_none() => impressora = Some(valor.clone()),
            "--lote" if tamanho.is_none() => {
                tamanho = Some(
                    valor
                        .parse::<usize>()
                        .map_err(|_| "--lote deve ser um inteiro positivo")?,
                )
            }
            "--saida" if diretorio.is_none() => diretorio = Some(PathBuf::from(valor)),
            "--perfis" if perfis.is_none() => perfis = Some(PathBuf::from(valor)),
            "--ordem" if ordem.is_none() => ordem = Some(crate::analisar_ordem(valor)?),
            _ => return Err(format!("opção desconhecida ou repetida: {chave}")),
        }
        i += 2;
    }
    if !simular {
        return Err(MSG_SEM_SIMULAR.into());
    }
    let nome = impressora.ok_or("informe --impressora NOME")?;
    let tamanho = tamanho.ok_or("informe --lote K")?;
    let diretorio = diretorio.ok_or("informe --saida DIRETORIO")?;
    let ordem = ordem.unwrap_or_default();
    if tamanho == 0 {
        return Err("--lote deve ser maior que zero".into());
    }
    if !diretorio.is_dir() {
        return Err("o diretório de saída deve existir".into());
    }
    let fila = resolver_fila(&nome, perfis, ordem)?;
    let opcoes = Opcoes {
        tamanho,
        ordem,
        perfil: fila.perfil,
        mecanica: fila.mecanica,
        folhas_finais,
    };
    let documento = Documento::ler(Path::new(arquivo))?;
    let plano = geracao::planejar(&documento, &opcoes)?;
    let arquivos = geracao::gerar_arquivos(&documento, &plano, &opcoes, &diretorio)?;
    geracao::gravar_sem_sobrescrever(&arquivos)?;
    writeln!(
        saida,
        "IMPRESSÃO SIMULADA — {}\n{} páginas — {} lote(s)",
        fila.nome,
        documento.paginas(),
        plano.lotes.len()
    )
    .map_err(|e| e.to_string())?;
    if fila.de_perfil_salvo {
        writeln!(
            saida,
            "Mecânica simulada coerente com o perfil salvo de {}: isso confere a sequência, não a máquina.",
            fila.nome
        )
        .map_err(|e| e.to_string())?;
    }
    geracao::escrever_avisos(&documento, &plano, ordem, saida)?;
    writeln!(
        saida,
        "Gerados {} PDFs em {}. A impressão é simulada: nenhum arquivo foi enviado à impressora.",
        arquivos.len(),
        diretorio.display()
    )
    .map_err(|e| e.to_string())?;
    let lotes: Vec<Lote> = plano
        .lotes
        .iter()
        .map(|lote| Lote {
            paginas: lote
                .paginas
                .iter()
                .map(|numero| Pagina {
                    numero: *numero,
                    girar_180: false,
                })
                .collect(),
            passada1: lote.passada1.clone(),
            passada2: lote.passada2.clone(),
        })
        .collect();
    conduzir_lotes(&fila, lotes, &diretorio, entrada, saida)?;
    if !fila.de_perfil_salvo && !fila.mecanica.saida_face_impressa_para_cima {
        writeln!(saida, "Na gaveta, vire o maço antes de ler as folhas.")
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Imprime cada lote na impressora simulada: frente, gesto, Enter, verso e conferência.
fn conduzir_lotes(
    fila: &Fila,
    lotes: Vec<Lote>,
    diretorio: &Path,
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> Result<(), String> {
    let total = lotes.len();
    let mut job = Job::novo(SimBackend::novo(fila.mecanica), lotes.clone())
        .map_err(|e| format!("erro ao preparar a impressão simulada: {e:?}"))?;
    for (indice, lote) in lotes.iter().enumerate() {
        let numero = indice + 1;
        writeln!(
            saida,
            "\nLote {numero} de {total}: {} folha(s)",
            lote.passada1.len()
        )
        .map_err(|e| e.to_string())?;
        writeln!(saida, "  Frente: {}", listar(&lote.passada1)).map_err(|e| e.to_string())?;
        job.enviar_frente()
            .map_err(|e| format!("erro na frente: {e:?}"))?;
        let estado = job
            .atualizar()
            .map_err(|e| format!("erro ao consultar frente: {e:?}"))?;
        if estado != EstadoJob::AguardandoRecolocacao {
            return Err("a frente simulada ainda não terminou".into());
        }
        writeln!(
            saida,
            "  Frente simulada: envio e confirmação do spooler foram só simulados; nada foi à impressora."
        )
        .map_err(|e| e.to_string())?;
        let gesto = gesto_do_perfil(&fila.perfil);
        if fila.de_perfil_salvo {
            writeln!(
                saida,
                "  Pegue o maço como saiu, não embaralhe, e recoloque do mesmo jeito que na calibração."
            )
            .map_err(|e| e.to_string())?;
            if let Some(texto) = &fila.gesto_texto {
                writeln!(saida, "  {texto}").map_err(|e| e.to_string())?;
            }
        } else {
            writeln!(saida, "  {}", gesto_de_recolocacao(fila.mecanica, gesto))
                .map_err(|e| e.to_string())?;
        }
        write!(
            saida,
            "  Recoloque o maço e pressione Enter para simular o verso: "
        )
        .map_err(|e| e.to_string())?;
        saida.flush().map_err(|e| e.to_string())?;
        let mut confirmacao = String::new();
        entrada
            .read_line(&mut confirmacao)
            .map_err(|e| format!("erro ao ler a confirmação: {e}"))?;
        if !confirmacao.ends_with('\n') {
            return Err(format!(
                "impressão cancelada no lote {numero} de {total}; os PDFs continuam em {}",
                diretorio.display()
            ));
        }
        job.recolocar(gesto)
            .map_err(|e| format!("erro na recolocação: {e:?}"))?;
        writeln!(saida, "  Verso: {}", listar(&lote.passada2)).map_err(|e| e.to_string())?;
        job.enviar_verso()
            .map_err(|e| format!("erro no verso: {e:?}"))?;
        job.atualizar()
            .map_err(|e| format!("erro ao consultar verso: {e:?}"))?;
        let pilha = job.backend().simulador().folhas_para_leitura();
        let (corretas, folhas) = relatar_folhas(saida, &lote.paginas, &pilha)?;
        writeln!(
            saida,
            "  Resultado do lote: {corretas} de {folhas} folhas corretas (na simulação)"
        )
        .map_err(|e| e.to_string())?;
        if numero < total {
            writeln!(saida, "  Próximo lote: {} de {total}.", numero + 1)
                .map_err(|e| e.to_string())?;
        } else {
            writeln!(
                saida,
                "Todos os lotes foram conferidos na simulação ({total} no total). Isso não confere nenhuma impressora real."
            )
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Frase fixa do gesto de recolocação, com o lado conforme a saída da impressora.
fn gesto_de_recolocacao(mecanica: Mecanica, gesto: Recolocacao) -> String {
    let lado = if face_impressa_para_cima_na_entrada(mecanica, gesto) {
        "cima"
    } else {
        "baixo"
    };
    format!(
        "Pegue o maço como saiu, não embaralhe, e coloque na entrada com a face impressa para {lado}."
    )
}

/// Face impressa para cima quando o maço entra de novo: a saída da impressora, invertida
/// quando o gesto vira o maço. O simulador faz a mesma inversão em `recolocar`.
fn face_impressa_para_cima_na_entrada(mecanica: Mecanica, gesto: Recolocacao) -> bool {
    mecanica.saida_face_impressa_para_cima != gesto.virar_maco
}

/// Gesto de recolocação registrado no perfil da fila.
fn gesto_do_perfil(perfil: &PerfilImpressora) -> Recolocacao {
    Recolocacao {
        virar_maco: perfil.gesto_recolocar.virar_maco,
        ..Recolocacao::default()
    }
}

/// Resolve o nome da impressora: simuladas têm perfil embutido; outras leem o perfil salvo.
fn resolver_fila(
    nome: &str,
    perfis: Option<PathBuf>,
    ordem: OrdemPassadas,
) -> Result<Fila, String> {
    if let Some(virtual_) = ImpressoraVirtual::de_nome(nome) {
        return Ok(Fila {
            nome: virtual_.nome().to_string(),
            perfil: virtual_.perfil(),
            mecanica: virtual_.mecanica(),
            de_perfil_salvo: false,
            gesto_texto: None,
        });
    }
    let caminho = caminho_perfis(perfis)?;
    let lidos = carregar_perfis(&caminho)?;
    let entrada = lidos.obter(nome).ok_or_else(|| {
        format!(
            "a impressora '{nome}' não tem perfil salvo; rode 'plots calibrar {nome} --saida DIRETORIO' antes"
        )
    })?;
    let mecanica = mecanica_coerente(&entrada.perfil, ordem).ok_or_else(|| {
        format!(
            "o perfil salvo de '{nome}' não confere com nenhuma impressora simulada; refaça a calibração com 'plots calibrar {nome} --saida DIRETORIO'"
        )
    })?;
    Ok(Fila {
        nome: nome.to_string(),
        perfil: entrada.perfil,
        mecanica,
        de_perfil_salvo: true,
        gesto_texto: entrada.gesto_texto.clone(),
    })
}

/// Mecânica simulada coerente com um perfil salvo: a primeira, em ordem fixa, que confere a
/// sequência que o perfil gera. A ordem fixa começa pelas duas impressoras simuladas.
/// Isso confere a sequência, não a máquina real. Devolve `None` quando nenhuma confere.
pub(crate) fn mecanica_coerente(
    perfil: &PerfilImpressora,
    ordem: OrdemPassadas,
) -> Option<Mecanica> {
    let combinadas = (0u8..32).map(|mascara| Mecanica {
        saida_face_impressa_para_cima: mascara & 1 != 0,
        saida_no_topo: mascara & 2 != 0,
        imprime_face_para_cima: mascara & 4 != 0,
        entrada_pelo_topo: mascara & 8 != 0,
        giro_segunda_passada: mascara & 16 != 0,
    });
    let bases = if perfil.passada1_inversa {
        [Mecanica::traseira(), Mecanica::gaveta()]
    } else {
        [Mecanica::gaveta(), Mecanica::traseira()]
    };
    bases
        .into_iter()
        .chain(combinadas)
        .find(|mecanica| sequencia_confere(perfil, ordem, mecanica))
}

/// Simula um documento de referência com a mecânica e confere todas as folhas.
fn sequencia_confere(perfil: &PerfilImpressora, ordem: OrdemPassadas, mecanica: &Mecanica) -> bool {
    let lotes = match plano_documento(
        REFERENCIA_PAGINAS,
        REFERENCIA_FOLHAS_POR_LOTE,
        ordem,
        *perfil,
    ) {
        Ok(lotes) if !lotes.is_empty() => lotes,
        _ => return false,
    };
    let gesto = gesto_do_perfil(perfil);
    lotes.iter().all(|lote| {
        let mut sim = Simulador::novo(*mecanica);
        if sim.imprimir_frente(&lote.passada1).is_err()
            || sim.recolocar(gesto).is_err()
            || sim.imprimir_verso(&lote.passada2).is_err()
        {
            return false;
        }
        let pilha = sim.folhas_para_leitura();
        conferir_folhas(&pilha, &crate::pares_esperados(&lote.paginas))
            .iter()
            .all(|veredito| matches!(veredito, Veredito::Certa))
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn frase_de_nao_envio_acompanha_o_envio_desligado() {
        assert!(
            !crate::ENVIO_REAL_LIBERADO,
            "envio real ligado: revise FRASE_NAO_ENVIA e a recusa sem --simular antes"
        );
        assert!(crate::USO_GERAL.contains(crate::FRASE_NAO_ENVIA));
        assert_eq!(MSG_SEM_SIMULAR, crate::MSG_ENVIO_INDISPONIVEL);
    }

    fn perfil(p1: bool, p2: bool, giro: bool) -> PerfilImpressora {
        PerfilImpressora {
            passada1_inversa: p1,
            passada2_inversa: p2,
            verso_girar_180: giro,
            ..PerfilImpressora::default()
        }
    }

    #[test]
    fn todo_perfil_encontra_mecanica_coerente_nas_duas_ordens() {
        for mascara in 0u8..8 {
            for ordem in [OrdemPassadas::ParesPrimeiro, OrdemPassadas::ImparesPrimeiro] {
                let p = perfil(mascara & 1 != 0, mascara & 2 != 0, mascara & 4 != 0);
                assert!(mecanica_coerente(&p, ordem).is_some(), "{p:?} {ordem:?}");
            }
        }
    }

    #[test]
    fn perfis_embutidos_escolhem_a_propria_mecanica_simulada() {
        let traseira = ImpressoraVirtual::Traseira.perfil();
        let gaveta = ImpressoraVirtual::Gaveta.perfil();
        let ordem = OrdemPassadas::ParesPrimeiro;
        assert_eq!(
            mecanica_coerente(&traseira, ordem),
            Some(Mecanica::traseira())
        );
        assert_eq!(mecanica_coerente(&gaveta, ordem), Some(Mecanica::gaveta()));
    }

    #[test]
    fn frase_do_gesto_e_a_da_especificacao_com_e_sem_virar() {
        let virando = Recolocacao {
            virar_maco: true,
            ..Recolocacao::default()
        };
        let sem_virar = Recolocacao::default();
        assert_eq!(
            gesto_de_recolocacao(Mecanica::traseira(), virando),
            "Pegue o maço como saiu, não embaralhe, e coloque na entrada com a face impressa para baixo."
        );
        assert_eq!(
            gesto_de_recolocacao(Mecanica::traseira(), sem_virar),
            "Pegue o maço como saiu, não embaralhe, e coloque na entrada com a face impressa para cima."
        );
    }

    #[test]
    fn frase_do_gesto_bate_com_a_face_que_entra_no_simulador() {
        for mecanica in [Mecanica::traseira(), Mecanica::gaveta()] {
            for virar_maco in [false, true] {
                let gesto = Recolocacao {
                    virar_maco,
                    ..Recolocacao::default()
                };
                let lotes = plano_documento(
                    2,
                    1,
                    OrdemPassadas::ParesPrimeiro,
                    PerfilImpressora::default(),
                )
                .unwrap();
                let mut sim = Simulador::novo(mecanica);
                sim.imprimir_frente(&lotes[0].passada1).unwrap();
                sim.recolocar(gesto).unwrap();
                let para_cima = sim.pilha().iter().all(|f| f.face_a_para_cima);
                assert!(sim.pilha().iter().all(|f| f.face_a_para_cima == para_cima));
                let lado = if para_cima { "cima" } else { "baixo" };
                let frase = gesto_de_recolocacao(mecanica, gesto);
                assert!(
                    frase.contains(&format!("face impressa para {lado}.")),
                    "{mecanica:?} virar={virar_maco}: {frase}"
                );
            }
        }
    }

    #[test]
    fn resolver_fila_reconhece_atalhos_e_nomes_das_simuladas() {
        for nome in ["traseira", "Simulada — Traseira"] {
            let fila = resolver_fila(nome, None, OrdemPassadas::ParesPrimeiro).unwrap();
            assert_eq!(fila.nome, "Simulada — Traseira");
            assert!(!fila.de_perfil_salvo);
        }
        for nome in ["gaveta", "Simulada — Gaveta"] {
            let fila = resolver_fila(nome, None, OrdemPassadas::ParesPrimeiro).unwrap();
            assert_eq!(fila.nome, "Simulada — Gaveta");
            assert_eq!(fila.mecanica, Mecanica::gaveta());
        }
    }
}
