//! Leitura do registro de eventos de impressão do Windows, só para saber se um trabalho que sumiu
//! da fila falhou. O processador de impressão de alguns drivers (o "Microsoft Print To PDF", que
//! nos testes locais falhou com dois trabalhos juntos) grava o evento 372 e descarta o trabalho
//! sem nenhuma página: a fila só mostra que o trabalho sumiu. Nada aqui escreve no registro nem muda uma fila.

use std::time::{Duration, Instant};

/// O evento "o trabalho não pôde ser impresso" do processador de impressão.
const EVENTO_FALHA_DO_PROCESSADOR: u32 = 372;
/// Folga para o relógio do evento ficar um pouco antes do início do envio.
const FOLGA_MS: u128 = 1_000;
/// Prazo total da consulta: ilegível, desligado ou lento segue como se nada tivesse sido achado.
const PRAZO: Duration = Duration::from_millis(1_800);
/// Quantos eventos recentes no máximo olhar.
const MAXIMO_DE_EVENTOS: usize = 64;

/// A consulta XPath: só o evento 372 depois do início do envio.
fn consulta(desde_ms: u128) -> String {
    format!(
        "*[System[(EventID={EVENTO_FALHA_DO_PROCESSADOR}) and TimeCreated[timediff(@SystemTime) <= {}]]]",
        desde_ms + FOLGA_MS
    )
}

fn sem_escape(texto: &str) -> String {
    texto
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// O texto de um parâmetro do evento, já sem o escape do XML. `None` se o parâmetro não existe.
fn parametro(xml: &str, numero: u32) -> Option<String> {
    let abre = format!("<Param{numero}>");
    let inicio = xml.find(&abre)? + abre.len();
    let resto = &xml[inicio..];
    let fim = resto.find(&format!("</Param{numero}>"))?;
    Some(sem_escape(&resto[..fim]))
}

/// O XML de um evento é de uma falha do processador de impressão deste trabalho. O nome da fila é
/// o terceiro parâmetro e se compara inteiro; o nome do documento é o primeiro, e separa um 372
/// antigo da mesma fila do trabalho que imprimiu. Se o Windows não mostra o nome (parâmetro vazio
/// ou ausente), não há comparação e valem a fila e o tempo.
pub(crate) fn evento_e_falha_do_trabalho(xml: &str, fila: &str, documento: &str) -> bool {
    if !xml.contains(&format!("<EventID>{EVENTO_FALHA_DO_PROCESSADOR}</EventID>")) {
        return false;
    }
    if parametro(xml, 3).as_deref() != Some(fila) {
        return false;
    }
    match parametro(xml, 1) {
        Some(nome) if !nome.trim().is_empty() => nome == documento,
        _ => true,
    }
}

/// Decide pelo que o leitor devolve. `leitor` recebe a consulta XPath e o prazo, e devolve os XML
/// dos eventos, ou `None` se o registro não pôde ser lido.
pub(crate) fn trabalho_perdido_com(
    fila: &str,
    documento: &str,
    desde: Instant,
    leitor: impl FnOnce(&str, Duration) -> Option<Vec<String>>,
) -> bool {
    let consulta = consulta(desde.elapsed().as_millis());
    leitor(&consulta, PRAZO).is_some_and(|eventos| {
        eventos
            .iter()
            .any(|xml| evento_e_falha_do_trabalho(xml, fila, documento))
    })
}

/// Consulta o registro de verdade, sem passar de `PRAZO`.
#[cfg(windows)]
pub(crate) fn trabalho_perdido(fila: &str, documento: &str, desde: Instant) -> bool {
    trabalho_perdido_com(fila, documento, desde, sistema::eventos)
}

#[cfg(windows)]
mod sistema {
    use super::MAXIMO_DE_EVENTOS;
    use std::time::Duration;
    use windows::core::{w, PCWSTR};
    use windows::Win32::System::EventLog::{EvtClose, EvtNext, EvtQuery, EvtRender, EVT_HANDLE};

    /// `EvtQueryChannelPath`.
    const CAMINHO_DE_CANAL: u32 = 0x1;
    /// `EvtQueryReverseDirection`: do mais novo ao mais velho.
    const DO_MAIS_NOVO: u32 = 0x200;
    /// `EvtRenderEventXml`.
    const RENDERIZAR_XML: u32 = 1;

    struct Alca(EVT_HANDLE);
    impl Drop for Alca {
        fn drop(&mut self) {
            // Garantia: a alça veio de uma chamada que deu certo e é fechada uma vez só.
            let _ = unsafe { EvtClose(self.0) };
        }
    }

    /// Lê numa thread própria, para um registro lento nunca passar do prazo.
    pub(super) fn eventos(consulta: &str, prazo: Duration) -> Option<Vec<String>> {
        let consulta = consulta.to_string();
        let (envia, recebe) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("registro-de-impressao".into())
            .spawn(move || {
                let _ = envia.send(ler(&consulta));
            })
            .ok()?;
        recebe.recv_timeout(prazo).ok().flatten()
    }

    fn ler(consulta: &str) -> Option<Vec<String>> {
        let consulta: Vec<u16> = consulta.encode_utf16().chain(std::iter::once(0)).collect();
        // Garantia: os dois textos terminam em zero e vivem durante a chamada.
        let resultado = unsafe {
            EvtQuery(
                None,
                w!("Microsoft-Windows-PrintService/Admin"),
                PCWSTR::from_raw(consulta.as_ptr()),
                CAMINHO_DE_CANAL | DO_MAIS_NOVO,
            )
        }
        .ok()?;
        let resultado = Alca(resultado);
        let mut eventos = Vec::new();
        let mut alcas = [0isize; 16];
        while eventos.len() < MAXIMO_DE_EVENTOS {
            let mut lidos = 0u32;
            // Garantia: `alcas` e `lidos` vivem durante a chamada.
            if unsafe { EvtNext(resultado.0, &mut alcas, 500, 0, &mut lidos) }.is_err() {
                break;
            }
            for bruta in alcas.iter().take(lidos as usize) {
                let alca = Alca(EVT_HANDLE(*bruta));
                if let Some(xml) = renderizar(&alca) {
                    eventos.push(xml);
                }
            }
            if lidos == 0 {
                break;
            }
        }
        Some(eventos)
    }

    fn renderizar(evento: &Alca) -> Option<String> {
        let (mut usados, mut propriedades) = (0u32, 0u32);
        // Primeira chamada: só descobre o tamanho (devolve erro de buffer pequeno).
        // Garantia: sem buffer, a função só preenche `usados`.
        let _ = unsafe {
            EvtRender(
                None,
                evento.0,
                RENDERIZAR_XML,
                0,
                None,
                &mut usados,
                &mut propriedades,
            )
        };
        if usados == 0 || usados > 1 << 20 {
            return None;
        }
        let mut buffer = vec![0u16; (usados as usize).div_ceil(2)];
        // Garantia: o buffer tem pelo menos `usados` bytes.
        unsafe {
            EvtRender(
                None,
                evento.0,
                RENDERIZAR_XML,
                usados,
                Some(buffer.as_mut_ptr().cast()),
                &mut usados,
                &mut propriedades,
            )
        }
        .ok()?;
        let fim = buffer.iter().position(|&u| u == 0).unwrap_or(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..fim]))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const DOCUMENTO: &str = "PlotS - lote 1 - frente";

    fn evento_de(id: u32, fila: &str, documento: &str) -> String {
        format!(
            "<Event xmlns='http://schemas.microsoft.com/win/2004/08/events/event'><System><EventID>{id}</EventID></System><UserData><PrintOnProcFailedEd><Param1>{documento}</Param1><Param2>Pessoa</Param2><Param3>{fila}</Param3><Param4>RAW</Param4><Param10>1234567890</Param10></PrintOnProcFailedEd></UserData></Event>"
        )
    }

    fn evento(id: u32, fila: &str) -> String {
        evento_de(id, fila, DOCUMENTO)
    }

    fn achou(fila: &str, eventos: Vec<String>) -> bool {
        trabalho_perdido_com(fila, DOCUMENTO, Instant::now(), |_, _| Some(eventos))
    }

    #[test]
    fn um_372_antigo_da_mesma_fila_com_outro_documento_nao_marca_o_trabalho() {
        let antigo = evento_de(372, "A", "PlotS - lote 2 - verso");
        assert!(!achou("A", vec![antigo.clone()]));
        // O do próprio documento, mesmo entre antigos, marca.
        assert!(achou("A", vec![antigo, evento_de(372, "A", DOCUMENTO)]));
    }

    #[test]
    fn nome_do_documento_oculto_pelo_windows_segue_pela_fila_e_pelo_tempo() {
        assert!(achou("A", vec![evento_de(372, "A", "")]));
        let sem_param1 = evento_de(372, "A", "").replace("<Param1></Param1>", "");
        assert!(achou("A", vec![sem_param1]));
    }

    #[test]
    fn o_evento_372_da_fila_exata_marca_o_trabalho_como_perdido() {
        assert!(achou(
            "PlotSim — Uso A",
            vec![evento(372, "PlotSim — Uso A")]
        ));
        // Outra fila, nome com sufixo, outro evento ou registro sem eventos: nada.
        assert!(!achou(
            "PlotSim — Uso A",
            vec![evento(372, "PlotSim — Uso B")]
        ));
        assert!(!achou(
            "PlotSim — Uso A",
            vec![evento(372, "PlotSim — Uso AB")]
        ));
        assert!(!achou(
            "PlotSim — Uso A",
            vec![evento(307, "PlotSim — Uso A")]
        ));
        assert!(!achou("PlotSim — Uso A", vec![]));
        // Entre vários eventos, basta um da fila.
        assert!(achou("B", vec![evento(372, "A"), evento(372, "B")]));
    }

    #[test]
    fn registro_ilegivel_ou_desligado_segue_como_trabalho_concluido() {
        assert!(!trabalho_perdido_com(
            "A",
            DOCUMENTO,
            Instant::now(),
            |_, _| None
        ));
    }

    #[test]
    fn a_consulta_limita_ao_evento_372_depois_do_inicio_com_folga() {
        let mut vista = String::new();
        let desde = Instant::now() - Duration::from_millis(5_000);
        trabalho_perdido_com("A", DOCUMENTO, desde, |consulta, prazo| {
            vista = consulta.to_string();
            assert!(prazo <= Duration::from_secs(2));
            None
        });
        assert!(vista.contains("EventID=372"), "{vista}");
        let ms: u128 = vista
            .split("<= ")
            .nth(1)
            .and_then(|r| r.split(']').next())
            .and_then(|n| n.parse().ok())
            .expect("limite de tempo na consulta");
        assert!((6_000..6_500).contains(&ms), "{vista}");
    }

    #[test]
    fn nome_com_caracteres_escapados_no_xml_compara_inteiro() {
        assert!(achou("A & B", vec![evento(372, "A &amp; B")]));
    }

    /// Lê o registro de verdade; só tem o que achar numa máquina onde esse evento já ocorreu.
    #[cfg(windows)]
    #[test]
    #[ignore = "lê o registro de eventos do Windows desta máquina"]
    fn leitura_real_do_registro_nao_trava_nem_passa_do_prazo() {
        let inicio = Instant::now();
        let _ = trabalho_perdido(
            "fila que nao existe",
            DOCUMENTO,
            inicio - Duration::from_secs(86_400),
        );
        assert!(inicio.elapsed() < Duration::from_secs(2));
        let eventos = sistema::eventos(&consulta(86_400_000), PRAZO).expect("registro legível");
        println!("{} eventos 372 nas últimas 24 h", eventos.len());
    }
}
