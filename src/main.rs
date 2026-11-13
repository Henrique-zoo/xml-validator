//! CLI de validação offline dos documentos XML do SDP.
//! O catálogo é carregado uma vez por execução; cada arquivo tem seu próprio
//! interner e mapas, liberados antes de carregar o documento seguinte.

use std::{
    ffi::OsString,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};
use xml_transformer::{interner::Interner, validation::Validador, xml::reader::parse_atracacoes};

/// Contrato da linha de comando, também apresentado em erros de uso.
const USO: &str = "Uso: xml_transformer [--sem-cores] <arquivo.xml> [outros.xml ...]\n\
Valida arquivos localmente, sem modificá-los. Use -- antes de nomes iniciados por '-'.\n\
Saída: 0 = válido; 1 = erro semântico; 2 = uso, I/O ou parsing. Em lote, prevalece o maior código.";

/// Opções já verificadas, preservando caminhos não Unicode suportados pelo SO.
struct Opcoes {
    /// Documentos processados sequencialmente na ordem recebida.
    arquivos: Vec<PathBuf>,
    /// Desativa ANSI mesmo se stderr estiver conectado a um terminal.
    sem_cores: bool,
}

/// Interpreta opções em qualquer posição; `--` encerra sua interpretação.
/// `None` representa ajuda, que não carrega catálogos nem arquivos.
fn opcoes(args: impl IntoIterator<Item = OsString>) -> Result<Option<Opcoes>, String> {
    let mut arquivos = Vec::new();
    let mut sem_cores = false;
    let mut literal = false;
    for arg in args {
        if !literal {
            if arg == "--" {
                literal = true;
                continue;
            }
            if arg == "--help" || arg == "-h" {
                return Ok(None);
            }
            if arg == "--sem-cores" {
                sem_cores = true;
                continue;
            }
            if arg.to_string_lossy().starts_with('-') {
                return Err(format!("opção desconhecida: {}", arg.to_string_lossy()));
            }
        }
        arquivos.push(PathBuf::from(arg));
    }
    if arquivos.is_empty() {
        return Err("informe ao menos um arquivo XML".into());
    }
    Ok(Some(Opcoes {
        arquivos,
        sem_cores,
    }))
}

/// Executa o lote e devolve seu código de saída agregado.
fn main() -> ExitCode {
    let opcoes = match opcoes(std::env::args_os().skip(1)) {
        Ok(None) => {
            println!("{USO}");
            return ExitCode::SUCCESS;
        }
        Ok(Some(opcoes)) => opcoes,
        Err(erro) => {
            eprintln!("{erro}\n{USO}");
            return ExitCode::from(2);
        }
    };
    let validador = match Validador::com_dados_incorporados() {
        Ok(validador) => validador,
        Err(erro) => {
            eprintln!("{erro}");
            return ExitCode::from(2);
        }
    };
    let cores = !opcoes.sem_cores && io::stderr().is_terminal();
    let mut saida = io::BufWriter::new(io::stdout().lock());
    let mut diagnosticos = io::BufWriter::new(io::stderr().lock());
    let mut codigo = 0;
    for caminho in &opcoes.arquivos {
        match validar_arquivo(caminho, &validador, cores, &mut saida, &mut diagnosticos) {
            Ok(resultado) => codigo = codigo.max(resultado),
            Err(erro) => {
                codigo = 2;
                if writeln!(diagnosticos, "{}: {erro}", caminho.display()).is_err() {
                    break;
                }
            }
        }
    }
    if saida.flush().is_err() || diagnosticos.flush().is_err() {
        codigo = 2;
    }
    ExitCode::from(codigo)
}

/// Lê um documento e apresenta seus diagnósticos antes de liberar a fonte.
///
/// # Errors
/// Propaga falhas de I/O. Erros de parsing são impressos com spans e retornam
/// código 2; erros semânticos são acumulados e retornam 1. O sucesso retorna 0.
fn validar_arquivo(
    caminho: &Path,
    validador: &Validador,
    cores: bool,
    saida: &mut impl Write,
    diagnosticos: &mut impl Write,
) -> io::Result<u8> {
    let texto = std::fs::read_to_string(caminho)?;
    let mut interner = Interner::new();
    let parsed = match parse_atracacoes(&texto, &mut interner) {
        Ok(parsed) => parsed,
        Err(erro) => {
            writeln!(diagnosticos, "{}:\n{erro}\n", caminho.display())?;
            return Ok(2);
        }
    };
    match validador.validar_atracacoes(&parsed, &interner) {
        Ok(()) => {
            writeln!(
                saida,
                "{}: documento válido para as regras locais ({} atracação/atracações).",
                caminho.display(),
                parsed.atracacoes.len()
            )?;
            Ok(0)
        }
        Err(erros) => {
            writeln!(diagnosticos, "{}:", caminho.display())?;
            for erro in &erros {
                if cores {
                    writeln!(diagnosticos, "{}\n", erro.colorido())?;
                } else {
                    writeln!(diagnosticos, "{erro}\n")?;
                }
            }
            writeln!(
                diagnosticos,
                "{}: {} diagnóstico(s) principal(is); alternativas são escolhas, não erros cumulativos.",
                caminho.display(),
                erros.len()
            )?;
            Ok(1)
        }
    }
}
