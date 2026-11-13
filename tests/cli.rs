//! Exercita o executável real: lote, caminhos, diagnósticos e códigos de saída.

use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

/// Diretório exclusivo, removido ao terminar o teste que o criou.
struct Arquivos(PathBuf);
impl Arquivos {
    /// Cria um espaço temporário isolado inclusive entre testes paralelos.
    fn new() -> Self {
        static SEQUENCIA: AtomicUsize = AtomicUsize::new(0);
        let caminho = std::env::temp_dir().join(format!(
            "sdp-cli-{}-{}",
            std::process::id(),
            SEQUENCIA.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&caminho).unwrap();
        Self(caminho)
    }
    /// Materializa a entrada sintética do teste.
    fn xml(&self, nome: &str, texto: &str) -> PathBuf {
        let caminho = self.0.join(nome);
        fs::write(&caminho, texto).unwrap();
        caminho
    }
}
impl Drop for Arquivos {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

/// Instancia o binário compilado pelo Cargo para este pacote.
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_xml_transformer"))
}

#[test]
fn ajuda_e_erros_de_uso_nao_dependem_de_arquivo() {
    let ajuda = cli().arg("--help").output().unwrap();
    assert!(ajuda.status.success());
    assert!(String::from_utf8(ajuda.stdout).unwrap().contains("Uso:"));
    for args in [vec![], vec!["--inexistente"], vec!["--sem-cores"]] {
        assert_eq!(cli().args(args).output().unwrap().status.code(), Some(2));
    }
}

#[test]
fn aceita_caminhos_com_espacos_e_opcoes_em_qualquer_posicao() {
    let arquivos = Arquivos::new();
    let caminho = arquivos.xml("atracação válida.xml", "<Movimento xmlns=\"SDPv2.xsd\"/>");
    let original = fs::read(&caminho).unwrap();
    for antes in [true, false] {
        let mut cmd = cli();
        if antes {
            cmd.arg("--sem-cores");
        }
        cmd.arg(&caminho);
        if !antes {
            cmd.arg("--sem-cores");
        }
        let resultado = cmd.output().unwrap();
        assert_eq!(resultado.status.code(), Some(0));
        assert!(resultado.stderr.is_empty());
        assert!(
            String::from_utf8(resultado.stdout)
                .unwrap()
                .contains("documento válido")
        );
    }
    assert_eq!(fs::read(caminho).unwrap(), original);
}

#[test]
fn lote_continua_apos_erro_de_parsing_e_preserva_localizacao() {
    let arquivos = Arquivos::new();
    let malformado = arquivos.xml(
        "quebrado.xml",
        "<Movimento xmlns=\"SDPv2.xsd\">\n<Atracacao>\n</Movimento>",
    );
    let valido = arquivos.xml("valido.xml", "<Movimento xmlns=\"SDPTup.xsd\"/>");
    let resultado = cli()
        .arg("--sem-cores")
        .arg(&malformado)
        .arg(&valido)
        .output()
        .unwrap();
    assert_eq!(resultado.status.code(), Some(2));
    assert!(
        String::from_utf8(resultado.stdout)
            .unwrap()
            .contains("valido.xml")
    );
    let diagnostico = String::from_utf8(resultado.stderr).unwrap();
    assert!(diagnostico.contains("quebrado.xml"));
    assert!(diagnostico.contains("Erro de parsing XML"));
    assert!(!diagnostico.contains('\x1b'));
}

#[test]
fn lote_continua_apos_falha_de_io() {
    let arquivos = Arquivos::new();
    let valido = arquivos.xml("valido.xml", "<Movimento xmlns=\"SDPv2.xsd\"/>");
    let resultado = cli()
        .arg(arquivos.0.join("ausente.xml"))
        .arg(valido)
        .output()
        .unwrap();
    assert_eq!(resultado.status.code(), Some(2));
    assert!(
        String::from_utf8(resultado.stdout)
            .unwrap()
            .contains("valido.xml")
    );
    assert!(
        String::from_utf8(resultado.stderr)
            .unwrap()
            .contains("ausente.xml")
    );
}

#[test]
fn marcador_literal_admite_nome_iniciado_por_hifen() {
    let arquivos = Arquivos::new();
    arquivos.xml("--sem-cores", "<Movimento xmlns=\"SDPv2.xsd\"/>");
    let resultado = cli()
        .current_dir(&arquivos.0)
        .args(["--", "--sem-cores"])
        .output()
        .unwrap();
    assert!(resultado.status.success());
}

#[cfg(feature = "local-tests")]
mod arquivos_locais {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/local/cli.rs"));
}
