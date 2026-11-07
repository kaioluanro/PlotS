//! Estimativa de tinta e de custo de uma impressão: seleção de páginas, cobertura CMYK,
//! perfis de tinta, configuração e totais.
//!
//! O crate é puro: não lê nem grava arquivos, não chama conversores de PDF e não depende da
//! interface. Quem guarda os dados é a camada de persistência do aplicativo.

mod cobertura;
mod configuracao;
mod custo;
mod perfil;
mod selecao;

#[cfg(test)]
mod testes;

pub use cobertura::{cobertura_de_bgra, cobertura_de_pam, Cobertura, ErroCobertura};
pub use configuracao::{
    ConfiguracaoDeTinta, ErroConfiguracao, ModoDeCor, Qualidade, TamanhoDaImagem, DPIS_SUPORTADOS,
    PERFIL_PADRAO,
};
pub use custo::{
    cartucho, custo_do_consumo, estimativa_da_pagina, estimativa_da_selecao,
    estimativa_do_documento, ml_da_pagina, pagina_mais_cara, Consumo, Custo, ErroCusto,
    EstadoDoCartucho, Estimativa, AREA_A4_MM2,
};
pub use perfil::{
    ErroPerfil, PerfilDeTinta, PerfisDeTinta, LIMITE_FATOR_QUALIDADE, LIMITE_ID_BYTES,
    LIMITE_NOME_BYTES, LIMITE_PRECO_ML, LIMITE_RENDIMENTO_PAGINAS, LIMITE_VOLUME_ML,
    NOMES_DOS_CANAIS,
};
pub use selecao::{
    selecao_de_texto, ErroSelecao, SelecaoDePaginas, LIMITE_NUMEROS, LIMITE_TEXTO_BYTES,
};
