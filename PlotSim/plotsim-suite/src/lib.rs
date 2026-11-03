//! Suíte ponta a ponta do simulador: documentos de teste, matriz de cenários, bancada física,
//! execução pelo spooler e relatório.

pub mod bancada;
pub mod documento;
#[cfg(windows)]
pub mod execucao;
pub mod matriz;
pub mod relatorio;
