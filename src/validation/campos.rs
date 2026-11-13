//! Regras de formato com diagnóstico compartilhadas por cargas e detalhamentos.
//! Helpers puros permanecem em `helper`; este módulo resolve alvos da fonte.

/// Verifica o comprimento do CE opcional; a aplicabilidade pertence à entidade.
pub(super) fn validar_ce_mercante(
    context: &mut super::context::ValidationContext<'_, '_>,
    valor: &str,
    elemento: crate::xml::reader::ElementId,
) {
    if let Err(erro) = super::helper::validar_comprimento_exato::<15>(valor) {
        context.erro_nos_valores(
            "<CEMercante> deve conter quinze caracteres quando informado",
            super::regras_de_negocio::CE_MERCANTE,
            [(elemento, erro.to_string())],
        );
    }
}

/// Verifica as 13 posições da representação textual de um peso já decodificado.
pub(super) fn validar_formato_peso(
    context: &mut super::context::ValidationContext<'_, '_>,
    elemento: crate::xml::reader::ElementId,
) {
    let texto = context.valor_original(elemento);
    if texto.len() > 13 {
        context.erro_nos_valores(
            "a representação do peso ultrapassa 13 posições",
            super::regras_de_negocio::pendentes::FORMATO_PESOS,
            [(
                elemento,
                format!("este peso ocupa {} posições; use no máximo 13", texto.len()),
            )],
        );
    }
}
