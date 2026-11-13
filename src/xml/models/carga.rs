//! Modelos XML das cargas movimentadas em uma atracação.
//!
//! O lifetime `'xml` marca textos emprestados da entrada. Identificadores
//! repetitivos usam `TextoId`, que requer o mesmo interner usado na leitura.

use crate::{
    interner::TextoId,
    xml::models::{
        enums::{NaturezaCarga, NavegacaoCarga, Sentido, TipoOperacaoCarga},
        peso::Peso,
    },
};

/// Dados de uma ocorrência `<Carga>` do documento SDP.
///
/// O tipo espelha a representação XML. Condições entre os campos — por
/// exemplo, quais identificadores de origem são válidos para cada sentido —
/// são verificadas posteriormente pela camada de validação.
/// Descrições como “oito algarismos” indicam o formato esperado; o tipo `&str`
/// preserva o texto mesmo quando ele não o satisfaz, para permitir diagnósticos.
/// `None` em navegação é tag ausente e difere de
/// `Some(NavegacaoCarga::NaoInformada)`, que representa o código explícito zero.
#[derive(Debug)]
pub struct Carga<'xml> {
    /// CNPJ do operador responsável pela movimentação.
    pub cnpj_operador: TextoId,
    /// Código da carga informado pela instalação.
    pub codigo_carga: TextoId,
    /// Código NCM/SH8 de oito algarismos da mercadoria, quando aplicável.
    pub codigo_ncm: Option<TextoId>,
    /// `Some(true)` para `C` (cheio) e `Some(false)` para `V` (vazio).
    /// `None` preserva a ausência da tag, sem presumir o estado do contêiner.
    pub cheio_vazio: Option<bool>,
    /// Número do conhecimento eletrônico Mercante, quando informado.
    pub ce_mercante: Option<&'xml str>, // deprecated
    /// Forma física ou natureza da carga.
    pub natureza_carga: NaturezaCarga,
    /// Sentido da movimentação: embarque, desembarque ou não informado.
    pub sentido: Option<Sentido>,
    /// Modalidade de navegação; omitida nas operações auxiliares previstas pelo leiaute.
    pub navegacao: Option<NavegacaoCarga>,

    /// Código ISO alfa-2 do país de origem, quando a origem é um porto.
    pub bigrama_pais_origem: Option<TextoId>,
    /// Código de três letras do porto de origem.
    pub trigrama_porto_origem: Option<TextoId>,
    /// Código TUP da instalação de origem, quando a origem é um terminal privado.
    pub codigo_tup_origem: Option<TextoId>,
    /// Código da área operacional de origem, aplicável ao sentido 2.
    pub codigo_area_origem: Option<TextoId>,

    /// Código ISO alfa-2 do país de destino, quando o destino é um porto.
    pub bigrama_pais_destino: Option<TextoId>,
    /// Código de três letras do porto de destino.
    pub trigrama_porto_destino: Option<TextoId>,
    /// Código TUP da instalação de destino, quando o destino é um terminal privado.
    pub codigo_tup_destino: Option<TextoId>,
    /// Código da área operacional de destino, aplicável ao sentido 1.
    pub codigo_area_destino: Option<TextoId>,

    /// Quantidade de unidades, usada para carga geral (`NaturezaCarga = 3`).
    pub quantidade: Option<u32>,
    /// Peso informado no XML em toneladas e armazenado em quilogramas exatos.
    pub peso_carga_bruta: Peso,
    /// Parcela do peso bruto pertencente à autorizada do TUP ou ao seu grupo.
    /// O campo não faz parte do leiaute de porto organizado.
    pub peso_carga_bruta_propria: Option<Peso>, // deprecated
    /// Parcela do peso bruto pertencente a terceiros em um documento TUP.
    /// O campo não faz parte do leiaute de porto organizado.
    pub peso_carga_bruta_terceiros: Option<Peso>, // deprecated
    /// Tipo de operação realizada sobre a carga.
    pub tipo_operacao_carga: TipoOperacaoCarga,
    /// Código da classe de produto perigoso segundo o IMDG.
    pub imdg_code: u8,
    /// Contêineres detalhados para operações que exigem essa informação.
    /// O reader usa `None` quando não existem blocos; uma ocorrência descreve
    /// uma mercadoria e não necessariamente um contêiner físico distinto.
    pub carga_conteinerizada: Option<Vec<CargaConteinerizada<'xml>>>,
}

/// Detalhamento de uma ocorrência `<CargaConteinerizada>`.
///
/// Os campos opcionais permitem preservar a representação de contêiner vazio
/// e diagnosticar ausências no cheio. Este tipo não certifica a validade do
/// identificador, a existência da mercadoria ou as relações entre os pesos.
#[derive(Debug)]
pub struct CargaConteinerizada<'xml> {
    /// Código da mercadoria conteinerizada, exigido para contêiner cheio.
    pub codigo_carga_conteinerizada: Option<TextoId>,
    /// Número do conhecimento eletrônico Mercante, quando informado.
    pub ce_mercante: Option<&'xml str>, // deprecated
    /// Peso informado no XML em toneladas e armazenado em quilogramas exatos.
    pub peso_carga_liquida: Option<Peso>,
    /// Parcela própria do peso líquido em um documento TUP.
    pub peso_carga_liquida_propria: Option<Peso>, // deprecated
    /// Parcela de terceiros do peso líquido em um documento TUP.
    pub peso_carga_liquida_terceiros: Option<Peso>, // deprecated
    /// Identificador do contêiner proveniente da tag `<IDConteiner>`.
    pub id_conteiner: TextoId,
    /// Código da classe de produto perigoso, quando aplicável.
    pub imdg_code: Option<u8>,
}
