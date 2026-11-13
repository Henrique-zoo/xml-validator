//! Regras locais de coletas de resíduos — Manual §9.2.1, pp. 40–42.
//!
//! # Checklist
//!
//! - [x] relacionar a flag à presença de pelo menos um certificado;
//! - [x] exigir ao menos um item por certificado;
//! - [x] validar CNPJs compactos e unidades 1 (m³) ou 2 (kg);
//! - [x] limitar os decimais a onze algarismos, com até três casas;
//! - [x] preservar a independência entre coleta e tipo de operação da atracação.
//!
//! Campos incondicionalmente obrigatórios e decimais são lidos pelo reader.
//! Não se inferem restrições temporais ou comparações de quantidades ausentes
//! do manual, nem vínculos de cadastro GISIS.

use super::{
    context::{RotuloDiagnostico, ValidationContext},
    helper::{validar_cnpj, validar_comprimento_exato},
    regras_de_negocio::pendentes::COLETA_RESIDUOS,
};
use crate::xml::reader::ParsedAtracacoes;

/// Valida certificados e itens com os mapas planos do mesmo documento.
pub(super) fn validar_residuos(
    context: &mut ValidationContext<'_, '_>,
    parsed: &ParsedAtracacoes<'_>,
) {
    for (a, m) in parsed.atracacoes_with_source() {
        let mapas = &parsed.sdp_source_map.coletas_residuos[m.coletas_residuos.range()];
        assert_eq!(
            a.coletas_residuos.len(),
            mapas.len(),
            "coletas e mapas devem estar alinhados"
        );
        if a.flag_possui_coleta_residuo && mapas.is_empty() {
            context.erro_com_detalhes(
                "a flag de coleta exige ao menos um bloco <ColetaResiduo>",
                COLETA_RESIDUOS,
                [
                    RotuloDiagnostico::no_valor(
                        m.flag_possui_coleta_residuo,
                        "a flag informa que houve coleta",
                    ),
                    RotuloDiagnostico::antes_tag_fechamento(
                        m.element,
                        "ajuda: adicione o certificado ou corrija a flag",
                    ),
                ],
            );
        } else if !a.flag_possui_coleta_residuo && !mapas.is_empty() {
            let mut labels = vec![RotuloDiagnostico::no_valor(
                m.flag_possui_coleta_residuo,
                "a flag informa que não houve coleta",
            )];
            labels.extend(
                mapas.iter().take(3).map(|m| {
                    RotuloDiagnostico::na_tag_abertura(m.element, "certificado informado")
                }),
            );
            context.erro_com_detalhes(
                "a flag de coleta e os certificados informados são incompatíveis",
                COLETA_RESIDUOS,
                labels,
            );
        }
        for (coleta, mapa) in a.coletas_residuos.iter().zip(mapas) {
            for (tag, valor, id) in [
                ("CNPJColetor", coleta.cnpj_coletor, mapa.cnpj_coletor),
                ("CNPJReceptor", coleta.cnpj_receptor, mapa.cnpj_receptor),
            ] {
                let erro = validar_comprimento_exato::<14>(valor)
                    .err()
                    .map(|e| e.to_string())
                    .or_else(|| validar_cnpj(valor).err().map(|(_, mensagem)| mensagem));
                if let Some(erro) = erro {
                    context.erro_nos_valores(
                        format!("<{tag}> deve conter um CNPJ válido sem máscara"),
                        COLETA_RESIDUOS,
                        [(id, erro)],
                    );
                }
            }
            let itens = &parsed.sdp_source_map.residuos[mapa.residuos.range()];
            assert_eq!(
                coleta.residuos.len(),
                itens.len(),
                "resíduos e mapas devem estar alinhados"
            );
            if itens.is_empty() {
                context.erro_antes_tags_fechamento(
                    "a coleta exige ao menos um <Residuo>",
                    COLETA_RESIDUOS,
                    [(
                        mapa.element,
                        "ajuda: informe os resíduos associados ao certificado",
                    )],
                );
            }
            for (item, mapa_item) in coleta.residuos.iter().zip(itens) {
                if !matches!(item.tipo_unidade, 1 | 2) {
                    context.erro_nos_valores(
                        "<TipoUnidade> deve ser 1 (m³) ou 2 (kg)",
                        COLETA_RESIDUOS,
                        [(
                            mapa_item.tipo_unidade,
                            "informe uma das duas unidades admitidas",
                        )],
                    );
                }
                for id in [mapa_item.qtd_solicitado, mapa_item.qtd_coletado] {
                    let texto = context.valor_original(id);
                    if texto.bytes().filter(u8::is_ascii_digit).count() > 11 {
                        context.erro_nos_valores("a quantidade de resíduo ultrapassa onze algarismos", COLETA_RESIDUOS, [(id, "use até onze algarismos no total, incluindo até três casas decimais")]);
                    }
                }
            }
        }
    }
}

/// Testes privados opcionais, mantidos somente na instalação local.
#[cfg(all(test, feature = "local-tests"))]
mod testes_locais {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/local/validation/residuos.rs"
    ));
}
