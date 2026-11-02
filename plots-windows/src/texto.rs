//! Conversão de textos UTF-16 vindos do sistema, sem confiar no tamanho anunciado.
//!
//! Estas funções só olham para bytes e posições; não desreferenciam ponteiros.

/// Lê bytes em UTF-16 little-endian até o primeiro zero ou o fim do trecho.
///
/// Um byte final sobrando é ignorado. Surrogates isolados viram U+FFFD, sem pânico.
/// Os espaços do início e do fim são preservados, pois o nome da fila é usado como chave.
pub fn texto_utf16_le(bytes: &[u8]) -> String {
    let unidades: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|par| u16::from_le_bytes(par.try_into().unwrap_or_default()))
        .take_while(|&unidade| unidade != 0)
        .collect();
    String::from_utf16_lossy(&unidades)
}

/// Lê unidades UTF-16 já separadas até o primeiro zero ou o fim do trecho.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn texto_de_unidades(unidades: &[u16]) -> String {
    let ate_o_zero: Vec<u16> = unidades
        .iter()
        .copied()
        .take_while(|&unidade| unidade != 0)
        .collect();
    String::from_utf16_lossy(&ate_o_zero)
}

/// Posição de `endereco` dentro de um buffer que começa em `base` e tem `len` bytes.
///
/// Devolve `None` quando o endereço cai fora do buffer, inclusive antes do início.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn deslocamento_no_buffer(base: usize, len: usize, endereco: usize) -> Option<usize> {
    let deslocamento = endereco.checked_sub(base)?;
    if deslocamento < len {
        Some(deslocamento)
    } else {
        None
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn utf16(texto: &str) -> Vec<u8> {
        texto.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
    }

    #[test]
    fn converte_nome_com_acento_e_espaco_exatamente() {
        let nome = "Impressora Lançamento — Escritório";
        assert_eq!(texto_utf16_le(&utf16(nome)), nome);
    }

    #[test]
    fn preserva_espacos_nas_pontas() {
        assert_eq!(texto_utf16_le(&utf16("  Fila  ")), "  Fila  ");
    }

    #[test]
    fn para_no_primeiro_zero_e_ignora_o_resto_do_trecho() {
        let mut bytes = utf16("Gaveta");
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(&utf16("lixo"));
        assert_eq!(texto_utf16_le(&bytes), "Gaveta");
    }

    #[test]
    fn trecho_sem_zero_final_e_lido_ate_o_fim() {
        assert_eq!(texto_utf16_le(&utf16("Sem fim")), "Sem fim");
    }

    #[test]
    fn trecho_vazio_ou_so_zero_devolve_texto_vazio() {
        assert_eq!(texto_utf16_le(&[]), "");
        assert_eq!(texto_utf16_le(&[0, 0]), "");
    }

    #[test]
    fn byte_final_sobrando_nao_causa_erro() {
        let mut bytes = utf16("ab");
        bytes.push(0x41);
        assert_eq!(texto_utf16_le(&bytes), "ab");
    }

    #[test]
    fn surrogate_isolado_vira_substituto_sem_panico() {
        // 0xD800 sozinho é um surrogate alto sem par.
        let bytes = [0x00, 0xD8, 0x41, 0x00];
        assert_eq!(texto_utf16_le(&bytes), "\u{FFFD}A");
    }

    #[test]
    fn par_de_surrogates_valido_e_convertido() {
        let emoji = "\u{1F5A8}";
        assert_eq!(texto_utf16_le(&utf16(emoji)), emoji);
    }

    #[test]
    fn unidades_ja_separadas_param_no_zero() {
        let unidades: Vec<u16> = "Fila é 1".encode_utf16().chain([0, 0x41]).collect();
        assert_eq!(texto_de_unidades(&unidades), "Fila é 1");
        assert_eq!(texto_de_unidades(&[]), "");
    }

    #[test]
    fn deslocamento_dentro_do_buffer_e_aceito() {
        assert_eq!(deslocamento_no_buffer(100, 50, 100), Some(0));
        assert_eq!(deslocamento_no_buffer(100, 50, 149), Some(49));
    }

    #[test]
    fn deslocamento_fora_do_buffer_e_recusado() {
        assert_eq!(deslocamento_no_buffer(100, 50, 99), None);
        assert_eq!(deslocamento_no_buffer(100, 50, 150), None);
        assert_eq!(deslocamento_no_buffer(100, 50, usize::MAX), None);
        assert_eq!(deslocamento_no_buffer(usize::MAX, 0, 0), None);
    }
}
