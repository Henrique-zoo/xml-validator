//! Regras relacionais entre registros, sem alterar o documento.
//!
//! # Checklist — Manual §2.1, §2.2.1 e §2.2.4
//!
//! - [x] identificar atracações repetidas na mesma instalação, ano e berço;
//! - [x] identificar cargas idênticas salvo os pesos na mesma atracação;
//! - [x] limitar a soma de unidades de contêiner a 32.767 por atracação.
//!
//! As buscas usam tabelas hash de IDs e valores tipados, com custo esperado
//! linear no número de registros, além da ordenação dos itens de cada carga
//! para comparar detalhamentos sem depender da ordem XML. Vetores de ocorrências só são alocados para
//! chaves repetidas. A ordem dos diagnósticos segue a segunda ocorrência de
//! cada grupo, independentemente da ordem interna do `HashMap`.

use super::{
    context::{RotuloDiagnostico, ValidationContext},
    regras_de_negocio as regras,
};
use crate::{
    interner::TextoId,
    xml::{models::carga::Carga, reader::ParsedAtracacoes},
};
use chrono::Datelike;
use std::{
    collections::{HashMap, hash_map::Entry},
    hash::Hash,
};

/// Agrupa índices repetidos, armazenando apenas o primeiro índice das chaves únicas.
fn repeticoes<K: Eq + Hash>(chaves: impl Iterator<Item = K>) -> Vec<Vec<usize>> {
    let mut vistos = HashMap::new();
    let mut grupos: Vec<Vec<usize>> = Vec::new();
    for (indice, chave) in chaves.enumerate() {
        match vistos.entry(chave) {
            Entry::Vacant(entry) => {
                entry.insert((indice, None));
            }
            Entry::Occupied(mut entry) => {
                let (primeiro, grupo) = entry.get_mut();
                let grupo = *grupo.get_or_insert_with(|| {
                    grupos.push(vec![*primeiro]);
                    grupos.len() - 1
                });
                grupos[grupo].push(indice);
            }
        }
    }
    grupos
}

/// Características de uma mercadoria dentro de um contêiner, excetuados os pesos.
type ChaveConteiner<'a> = (TextoId, Option<TextoId>, Option<&'a str>, Option<u8>);

/// Identidade de uma carga para consolidação, sem copiar textos ou pesos.
#[derive(PartialEq, Eq, Hash)]
struct ChaveCarga<'a> {
    /// Identificação, representação e operação.
    carga: (
        TextoId,
        TextoId,
        Option<TextoId>,
        Option<bool>,
        Option<&'a str>,
        u32,
        Option<u32>,
        Option<u32>,
        u32,
        u8,
    ),
    /// Identificadores de país, porto, TUP e área nas duas extremidades.
    locais: [Option<TextoId>; 8],
    /// Quantidade declarada, que também caracteriza o registro.
    quantidade: Option<u32>,
    /// Detalhamentos em ordem canônica; a ordem das tags não altera a identidade.
    conteineres: Vec<ChaveConteiner<'a>>,
}

impl<'a> ChaveCarga<'a> {
    /// Extrai a chave sem incluir nenhum dos campos de peso.
    fn new(c: &Carga<'a>) -> Self {
        let mut conteineres: Vec<_> = c
            .carga_conteinerizada
            .iter()
            .flatten()
            .map(|d| {
                (
                    d.id_conteiner,
                    d.codigo_carga_conteinerizada,
                    d.ce_mercante,
                    d.imdg_code,
                )
            })
            .collect();
        conteineres.sort_unstable();
        Self {
            carga: (
                c.cnpj_operador,
                c.codigo_carga,
                c.codigo_ncm,
                c.cheio_vazio,
                c.ce_mercante,
                c.natureza_carga as u32,
                c.sentido.map(|v| v as u32),
                c.navegacao.map(|v| v as u32),
                c.tipo_operacao_carga as u32,
                c.imdg_code,
            ),
            locais: [
                c.bigrama_pais_origem,
                c.trigrama_porto_origem,
                c.codigo_tup_origem,
                c.codigo_area_origem,
                c.bigrama_pais_destino,
                c.trigrama_porto_destino,
                c.codigo_tup_destino,
                c.codigo_area_destino,
            ],
            quantidade: c.quantidade,
            conteineres,
        }
    }
}

/// Aplica unicidade por instalação e consolidação apenas dentro de cada atracação.
pub(super) fn validar_documento(
    context: &mut ValidationContext<'_, '_>,
    parsed: &ParsedAtracacoes<'_>,
) {
    let grupos = repeticoes(parsed.atracacoes.iter().map(|a| {
        (
            a.trigrama_porto,
            a.codigo_tup,
            a.numero_atracacao,
            a.local_atracacao,
            a.data_hora_atracacao.year(),
        )
    }));
    for grupo in grupos {
        let labels = grupo.iter().take(3).map(|&i| {
            RotuloDiagnostico::no_valor(
                parsed.sdp_source_map.atracacoes[i].numero_atracacao,
                "mesmo número, ano, local e instalação informante",
            )
        });
        context.erro_com_detalhes(
            format!(
                "{} atracações compartilham o mesmo identificador no ano e local",
                grupo.len()
            ),
            regras::pendentes::UNICIDADE_ATRACACAO,
            labels,
        );
    }
    for (a, mapa) in parsed.atracacoes_with_source() {
        let mapas = &parsed.sdp_source_map.cargas[mapa.cargas.range()];
        for grupo in repeticoes(a.cargas.iter().map(ChaveCarga::new)) {
            let labels = grupo.iter().take(3).map(|&i| {
                RotuloDiagnostico::na_tag_abertura(
                    mapas[i].element,
                    "registro com as mesmas características, excetuados os pesos",
                )
            });
            context.erro_com_detalhes(
                format!(
                    "{} cargas com características idênticas devem ser consolidadas",
                    grupo.len()
                ),
                regras::pendentes::CONSOLIDACAO_CARGAS,
                labels,
            );
        }
        let mut total = 0u64;
        let mut exemplos = Vec::new();
        for (c, m) in a.cargas.iter().zip(mapas) {
            let conteinerizada = c.cheio_vazio.is_some()
                || c.carga_conteinerizada.is_some()
                || context.codigo_conteiner_existe(context.texto(c.codigo_carga));
            if conteinerizada && let Some((quantidade, id)) = c.quantidade.zip(m.quantidade) {
                total += u64::from(quantidade);
                if exemplos.len() < 3 {
                    exemplos.push(RotuloDiagnostico::no_valor(
                        id,
                        "esta quantidade compõe o total de contêineres",
                    ));
                }
            }
        }
        if total > 32_767 {
            context.erro_com_detalhes(
                format!("a atracação informa {total} unidades de contêiner; o limite é 32.767"),
                regras::LIMITES_QUANTIDADE,
                exemplos,
            );
        }
    }
}
