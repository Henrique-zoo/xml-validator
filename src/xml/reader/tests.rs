//! Regressões da estrutura XML, dos valores internados e dos mapas de origem.
//!
//! Os testes da decodificação fornecem o interner ao reader e conferem os IDs
//! por entradas já ocupadas, sem inserir o texto esperado durante a asserção.
//! Textos repetidos compartilham IDs, mas preservam ocorrências e spans próprios.
//!
//! Os testes versionados usam apenas documentos sintéticos construídos no código.

use super::{document::read_document, *};
use crate::{
    interner::{Entry, Interner, TextoId},
    xml::models::documento::LeiauteSdp,
};

/// Consulta um texto que o reader já deve ter internado, sem inseri-lo.
///
/// # Panics
///
/// Falha na linha da chamada se o texto esperado não estiver na tabela.
#[track_caller]
fn interned_id<'xml>(interner: &mut Interner<'xml>, text: &'xml str) -> TextoId {
    match interner.entry(text) {
        Entry::Occupied(id) => id,
        Entry::Vacant { .. } => panic!("o reader deveria ter internado `{text}`"),
    }
}

/// Confere os recortes exatos dos tokens e das linhas lógicas em XML com CRLF.
#[test]
fn preserves_line_and_token_spans() {
    let source = concat!(
        "<?xml version=\"1.0\" ?>\r\n",
        "<Movimento>\r\n",
        "  <NumeroAtracacao>9062/2026</NumeroAtracacao>\r\n",
        "</Movimento>\r\n",
    );
    let document = read_document(source).expect("XML de teste deve ser válido");

    assert_eq!(document.source_map.lines.len(), 4);
    let line = &document.source_map.lines[2];
    let line_location = document.location(line.span);
    assert_eq!(line.kind, XmlLineKind::TagValueEndTag);
    assert_eq!(line_location.line, 3);
    assert_eq!(line_location.column, 3);
    assert_eq!(
        line.span.text(source),
        Some("<NumeroAtracacao>9062/2026</NumeroAtracacao>")
    );

    let tokens = &document.source_map.tokens[line.tokens.range()];
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].kind, XmlTokenKind::Tag);
    assert_eq!(tokens[0].span.text(source), Some("<NumeroAtracacao>"));
    assert_eq!(tokens[1].kind, XmlTokenKind::Value);
    assert_eq!(tokens[1].span.text(source), Some("9062/2026"));
    assert_eq!(tokens[2].kind, XmlTokenKind::EndTag);
    assert_eq!(tokens[2].span.text(source), Some("</NumeroAtracacao>"));
}

/// Verifica que nomes e namespaces da árvore emprestem a própria fonte.
#[test]
fn structural_symbols_borrow_the_original_xml() {
    let source = r#"<ns:Movimento xmlns="SDPv2.xsd"><ns:Atracacao/></ns:Movimento>"#;
    let document = read_document(source).expect("XML com prefixo deve ser estruturalmente válido");
    let root = document.roots[0];
    let name = document.name(root);
    let namespace = document.namespace(root).unwrap();
    let source_start = source.as_ptr() as usize;
    let source_end = source_start + source.len();

    assert_eq!(name, "Movimento");
    assert_eq!(namespace, "SDPv2.xsd");
    assert!((source_start..source_end).contains(&(name.as_ptr() as usize)));
    assert!((source_start..source_end).contains(&(namespace.as_ptr() as usize)));
}

/// Confere unidades lógicas e spans quando o XML não contém quebras físicas.
#[test]
fn builds_xml_logical_lines_in_a_document_without_newlines() {
    let source = concat!(
        "<?xml version=\"1.0\"?>",
        "<Movimento>",
        "<Atracacao>",
        "<NumeroAtracacao>9062/2026</NumeroAtracacao>",
        "<FlagPossuiColetaResiduo/>",
        "</Atracacao>",
        "</Movimento>",
    );
    let document = read_document(source).expect("XML compacto deve ser válido");

    assert_eq!(document.source_map.lines.len(), 7);
    assert_eq!(
        document
            .source_map
            .lines
            .iter()
            .map(|line| line.kind)
            .collect::<Vec<_>>(),
        vec![
            XmlLineKind::Declaration,
            XmlLineKind::Tag,
            XmlLineKind::Tag,
            XmlLineKind::TagValueEndTag,
            XmlLineKind::Tag,
            XmlLineKind::EndTag,
            XmlLineKind::EndTag,
        ]
    );
    assert!(
        document
            .source_map
            .lines
            .iter()
            .all(|line| document.location(line.span).line == 1)
    );
    assert_eq!(
        document.source_map.lines[3].span.text(source),
        Some("<NumeroAtracacao>9062/2026</NumeroAtracacao>")
    );
}

/// Confere que uma quebra física separe a abertura do fechamento posterior.
#[test]
fn physical_newline_ends_an_incomplete_xml_logical_line() {
    let source = "<Movimento><NumeroAtracacao>\n9062/2026</NumeroAtracacao></Movimento>";
    let document = read_document(source).expect("XML multilinha deve ser válido");

    assert_eq!(
        document
            .source_map
            .lines
            .iter()
            .map(|line| line.kind)
            .collect::<Vec<_>>(),
        vec![
            XmlLineKind::Tag,
            XmlLineKind::Tag,
            XmlLineKind::ValueEndTag,
            XmlLineKind::EndTag,
        ]
    );
    assert_eq!(document.location(document.source_map.lines[2].span).line, 2);
}





/// Exercita blocos opcionais, valores e mapas ausentes do exemplo principal.
#[test]
fn parses_optional_structures_from_the_xml_vocabulary() {
    let source = r#"<Movimento xmlns="SDPTup.xsd">
  <Atracacao>
<CodigoTUP>BRSC011</CodigoTUP>
<NumeroAtracacao>1/2026</NumeroAtracacao>
<NREscala>123456</NREscala>
<LocalAtracacao>PIB3</LocalAtracacao>
<IMON>9356141</IMON>
<NumeroInscricaoCapitania>1234567890</NumeroInscricaoCapitania>
<Navegacao>3</Navegacao>
<TipoOperacao>7</TipoOperacao>
<CnpjAgente>00728995001183</CnpjAgente>
<NacionalidadeArmador>1</NacionalidadeArmador>
<CnpjCodigoArmador>01341776000138</CnpjCodigoArmador>
<DataHoraChegada>2026-07-16T00:30:00</DataHoraChegada>
<DataHoraPrevistaAtracacao>2026-07-16T02:00:00</DataHoraPrevistaAtracacao>
<DataHoraAtracacao>2026-07-16T02:10:00</DataHoraAtracacao>
<DataHoraDesatracacao>2026-07-16T20:30:00</DataHoraDesatracacao>
<DataHoraInicioOperacao>2026-07-16T02:58:00</DataHoraInicioOperacao>
<DataHoraTerminoOperacao>2026-07-16T18:56:00</DataHoraTerminoOperacao>
<TemposDescontos>
  <Codigo>2</Codigo>
  <DataHoraInicio>2026-07-16T10:00:00</DataHoraInicio>
  <DataHoraTermino>2026-07-16T10:30:00</DataHoraTermino>
</TemposDescontos>
<QuantidadePassageirosEmbarcados>10</QuantidadePassageirosEmbarcados>
<QuantidadePassageirosDesembarcados>8</QuantidadePassageirosDesembarcados>
<QuantidadePassageirosTransito>3</QuantidadePassageirosTransito>
<FlagPossuiColetaResiduo>1</FlagPossuiColetaResiduo>
<Carga>
    <CnpjOperador>04471564000163</CnpjOperador>
    <CodigoCarga>22G1</CodigoCarga>
    <CodigoNCM>2203</CodigoNCM>
    <CheioVazio>C</CheioVazio>
    <CEMercante>072405387029379</CEMercante>
    <NaturezaCarga>3</NaturezaCarga>
    <Sentido>1</Sentido>
    <Navegacao>3</Navegacao>
    <CodigoTUPOrigem>BRSC011</CodigoTUPOrigem>
    <BigramaPaisDestino>BR</BigramaPaisDestino>
    <TrigramaPortoDestino>SUA</TrigramaPortoDestino>
    <Quantidade>1</Quantidade>
    <PesoCargaBruta>1.250</PesoCargaBruta>
    <PesoCargaBrutaPropria>0.250</PesoCargaBrutaPropria>
    <PesoCargaBrutaTerceiros>1.000</PesoCargaBrutaTerceiros>
    <TipoOperacaoCarga>17</TipoOperacaoCarga>
    <ImdgCode>1</ImdgCode>
    <CargaConteinerizada>
      <CodigoCargaConteinerizada>2203</CodigoCargaConteinerizada>
      <CEMercante>072405387029379</CEMercante>
      <PesoCargaLiquida>1.000</PesoCargaLiquida>
      <PesoCargaLiquidaPropria>0.250</PesoCargaLiquidaPropria>
      <PesoCargaLiquidaTerceiros>0.750</PesoCargaLiquidaTerceiros>
      <IDConteiner>ABCD1234567</IDConteiner>
      <ImdgCode>0</ImdgCode>
    </CargaConteinerizada>
</Carga>
  </Atracacao>
</Movimento>
"#;

    let mut interner = Interner::new();
    let parsed = parse_atracacoes(source, &mut interner).expect("tags inferidas devem ser aceitas");
    assert_eq!(parsed.leiaute, LeiauteSdp::Tup);
    let atracacao = &parsed.atracacoes[0];
    assert_eq!(atracacao.trigrama_porto, None);
    assert_eq!(
        atracacao.codigo_tup,
        Some(interned_id(&mut interner, "BRSC011"))
    );
    assert_eq!(atracacao.nr_escala, Some("123456"));
    assert_eq!(atracacao.imon, "9356141");
    assert_eq!(atracacao.numero_inscricao_capitania, Some("1234567890"));
    assert_eq!(atracacao.tempos_descontos.as_ref().unwrap()[0].codigo, 2);
    assert_eq!(atracacao.quantidade_passageiros_embarcados, Some(10));
    assert_eq!(atracacao.quantidade_passageiros_desembarcados, Some(8));
    assert_eq!(atracacao.quantidade_passageiros_transito, Some(3));
    assert!(atracacao.flag_possui_coleta_residuo);
    let carga = &atracacao.cargas[0];
    assert_eq!(
        carga.cnpj_operador,
        interned_id(&mut interner, "04471564000163")
    );
    assert_eq!(carga.codigo_carga, interned_id(&mut interner, "22G1"));
    assert_eq!(carga.codigo_ncm, Some(interned_id(&mut interner, "2203")));
    assert_eq!(carga.codigo_tup_origem, atracacao.codigo_tup);
    assert_eq!(
        carga.bigrama_pais_destino,
        Some(interned_id(&mut interner, "BR"))
    );
    assert_eq!(
        carga.trigrama_porto_destino,
        Some(interned_id(&mut interner, "SUA"))
    );
    assert_eq!(carga.bigrama_pais_origem, None);
    assert_eq!(carga.trigrama_porto_origem, None);
    assert_eq!(carga.codigo_area_origem, None);
    assert_eq!(carga.codigo_tup_destino, None);
    assert_eq!(carga.codigo_area_destino, None);
    assert_eq!(atracacao.cargas[0].peso_carga_bruta.quilogramas(), 1_250);
    assert_eq!(
        atracacao.cargas[0]
            .peso_carga_bruta_propria
            .unwrap()
            .quilogramas(),
        250
    );
    assert_eq!(
        atracacao.cargas[0]
            .peso_carga_bruta_terceiros
            .unwrap()
            .quilogramas(),
        1_000
    );
    let conteiner = &atracacao.cargas[0].carga_conteinerizada.as_ref().unwrap()[0];
    assert_eq!(conteiner.codigo_carga_conteinerizada, carga.codigo_ncm);
    assert_eq!(
        conteiner.peso_carga_liquida_propria.unwrap().quilogramas(),
        250
    );
    assert_eq!(
        conteiner
            .peso_carga_liquida_terceiros
            .unwrap()
            .quilogramas(),
        750
    );

    let atracacao_source = parsed.sdp_source_map.atracacoes[0];
    assert_eq!(atracacao_source.tempos_descontos.len(), 1);
    assert_eq!(atracacao_source.cargas.len(), 1);
    let carga_source = parsed.sdp_source_map.cargas[atracacao_source.cargas.range().start];
    assert_eq!(carga_source.conteineres.len(), 1);
    let conteiner_source =
        parsed.sdp_source_map.conteineres[carga_source.conteineres.range().start];
    let ncm_element = carga_source.codigo_ncm.unwrap();
    let mercadoria_element = conteiner_source.codigo_carga_conteinerizada.unwrap();
    assert_ne!(ncm_element, mercadoria_element);
    let ncm_span = parsed.element_value_span(ncm_element);
    let mercadoria_span = parsed.element_value_span(mercadoria_element);
    assert_ne!(ncm_span, mercadoria_span);
    assert_eq!(ncm_span.text(source), Some("2203"));
    assert_eq!(mercadoria_span.text(source), Some("2203"));
}
