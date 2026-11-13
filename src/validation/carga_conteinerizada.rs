//! Regras semânticas de uma ocorrência `<CargaConteinerizada>`.
//!
//! Os enunciados gerais dos diagnósticos são compartilhados por
//! [`crate::validation::regras_de_negocio`]; mensagens e labels descrevem cada ocorrência.
//!
//! Valida os campos internos do detalhamento, usando o estado de `CheioVazio`
//! fornecido pela carga pai. Relações entre o peso bruto e a soma dos pesos
//! líquidos pertencem ao módulo [`carga`](crate::validation::carga). Este módulo não altera os
//! dados: instruções de remoção aparecem somente nos diagnósticos.
//!
//! A função de entidade seleciona os campos presentes e as regras de cheio ou
//! vazio. As regras lexicais recebem texto e ID; a soma recebe três pares de
//! peso e ID, sem acessar o modelo. As regras conjuntas de presença/ausência
//! conservam o mapa, pois precisam inspecionar vários campos do detalhamento.
//!
//! # Fonte normativa
//!
//! O checklist deriva do *Manual do Usuário Externo do sistema SDP*, 5ª
//! edição, versão 1.0, dezembro de 2025: seções 2.2.6 (pp. 18–19), 3.2
//! (p. 20), 10 (pp. 43–45) e exemplos de contêiner vazio das seções 15.4 e
//! 15.5 (pp. 52–53).
//!
//! # Checklist normativo
//!
//! - [x] exigir detalhamento para cargas conteinerizadas, admitindo várias
//!   mercadorias para o mesmo identificador; a completude real da declaração
//!   não pode ser comprovada apenas com o XML;
//! - [x] exigir `IDConteiner` com exatamente 11 caracteres alfanuméricos ASCII;
//! - [x] quando presente, exigir que `CodigoCargaConteinerizada` contenha
//!   quatro algarismos ASCII, formato dos quatro primeiros algarismos do NCM;
//!   não há consulta à existência do código nem comparação com um cadastro;
//! - [x] para contêiner cheio, exigir a presença de
//!   `CodigoCargaConteinerizada` para cada mercadoria;
//! - [x] para contêiner vazio, manter o bloco com `IDConteiner`, mas remover
//!   `CodigoCargaConteinerizada`, `CEMercante`, pesos líquidos e `ImdgCode`,
//!   conforme a regra textual e os exemplos do Manual;
//! - [x] não exigir `CEMercante` no detalhamento de contêiner cheio: a antiga
//!   exigência por navegação foi retirada do projeto; quando presente, verificar
//!   quinze caracteres; no estado vazio, sua
//!   presença continua proibida;
//! - [x] exigir `PesoCargaLiquida` para mercadoria em contêiner cheio;
//! - [x] limitar a representação textual do peso a 13 posições; a conversão
//!   exata de toneladas e até três casas decimais já pertence ao reader;
//! - [x] exigir `ImdgCode` para cada mercadoria de cada contêiner e usar 0
//!   quando ela não for perigosa;
//! - [x] como regra adicional do projeto, quando as três parcelas existirem,
//!   exigir que `PesoCargaLiquidaPropria + PesoCargaLiquidaTerceiros` resulte
//!   em `PesoCargaLiquida`; essa igualdade não é atribuída ao Manual.
//!
//! O Manual não estabelece dígito verificador ISO 6346, unicidade de
//! `IDConteiner` nem regra geral de duplicidade dos blocos. Essas restrições não devem ser
//! implementadas sem outra fonte normativa.

use crate::{
    interner::TextoId,
    validation::carga::EstadoContainer,
    xml::{
        models::{carga::CargaConteinerizada, peso::Peso},
        reader::{CargaConteinerizadaSourceMap, ElementId},
    },
};

use super::{
    context::{RotuloDiagnostico, ValidationContext},
    helper::{validar_codigo_alfanumerico, validar_codigo_numerico},
    regras_de_negocio as regras,
};

/// Executa formato e presença/ausência dos campos de um detalhamento.
///
/// O `estado` vem de `<CheioVazio>` na carga pai e permite que as regras do
/// detalhamento adaptem seus diagnósticos e suas exigências a contêineres
/// cheios, vazios ou sem estado informado.
/// Quando o estado não foi informado, somente as verificações lexicais de
/// identificador e código são executadas, sem presumir cheio ou vazio.
///
/// # Panics
///
/// Pode entrar em pânico se os IDs do mapa não pertencerem ao documento do
/// contexto. Modelo e mapa devem corresponder à mesma ocorrência do reader.
pub(super) fn validar_carga_conteinerizada<'xml>(
    context: &mut ValidationContext<'_, 'xml>,
    carga_conteinerizada: &CargaConteinerizada<'xml>,
    source_map: &CargaConteinerizadaSourceMap,
    estado: EstadoContainer,
) {
    for id in [
        source_map.peso_carga_liquida,
        source_map.peso_carga_liquida_propria,
        source_map.peso_carga_liquida_terceiros,
    ]
    .into_iter()
    .flatten()
    {
        super::campos::validar_formato_peso(context, id);
    }
    if estado != EstadoContainer::Vazio
        && let Some((valor, id)) = carga_conteinerizada.ce_mercante.zip(source_map.ce_mercante)
    {
        super::campos::validar_ce_mercante(context, valor, id);
    }
    validar_id_container(
        context,
        carga_conteinerizada.id_conteiner,
        source_map.id_conteiner,
    );
    if let Some((codigo, elemento)) = carga_conteinerizada
        .codigo_carga_conteinerizada
        .zip(source_map.codigo_carga_conteinerizada)
    {
        validar_codigo_carga_conteinerizada(context, codigo, elemento);
    }
    match estado {
        EstadoContainer::Cheio => {
            validar_campos_carga_cheia(context, source_map);
            if let (Some(propria), Some(terceiros), Some(total)) = (
                carga_conteinerizada
                    .peso_carga_liquida_propria
                    .zip(source_map.peso_carga_liquida_propria),
                carga_conteinerizada
                    .peso_carga_liquida_terceiros
                    .zip(source_map.peso_carga_liquida_terceiros),
                carga_conteinerizada
                    .peso_carga_liquida
                    .zip(source_map.peso_carga_liquida),
            ) {
                validar_soma_cargas_liquidas(context, propria, terceiros, total);
            }
        }
        EstadoContainer::Vazio => validar_campos_carga_vazia(context, source_map),
        EstadoContainer::NaoInformado => (),
    }
}

/// Exige onze caracteres alfanuméricos ASCII em `<IDConteiner>`.
///
/// A restrição lexical é igual para contêineres cheios e vazios; portanto,
/// esta regra não recebe [`EstadoContainer`]. O estado pertence às validações
/// dos demais campos do detalhamento.
fn validar_id_container(
    context: &mut ValidationContext<'_, '_>,
    identificador: TextoId,
    elemento: ElementId,
) {
    if let Err(erro) = validar_codigo_alfanumerico::<11>(context.texto(identificador)) {
        context.erro_nos_valores(
            "o valor de <IDConteiner> deve conter exatamente onze caracteres alfanuméricos ASCII",
            regras::ID_CONTEINER,
            [(elemento, erro.to_string())],
        );
    }
}

/// Exige quatro algarismos ASCII no código SH4 da mercadoria conteinerizada.
///
/// A orquestração chama esta regra somente para um código presente. A ausência
/// da tag é tratada separadamente porque depende do estado da carga pai.
fn validar_codigo_carga_conteinerizada(
    context: &mut ValidationContext<'_, '_>,
    codigo: TextoId,
    elemento: ElementId,
) {
    if let Err(erro) = validar_codigo_numerico::<4>(context.texto(codigo)) {
        context.erro_nos_valores(
            "o valor de <CodigoCargaConteinerizada> deve conter exatamente quatro algarismos ASCII",
            regras::CODIGO_CARGA_CONTEINERIZADA,
            [(elemento, erro.to_string())],
        );
    }
}

/// Confere a soma das parcelas presentes (regra adicional do projeto).
///
/// A orquestração garante a presença dos três pesos e seus IDs antes da chamada.
/// A igualdade não é atribuída ao Manual e usa aritmética verificada; overflow
/// também gera incompatibilidade, sem substituir valores ausentes por zero.
fn validar_soma_cargas_liquidas(
    context: &mut ValidationContext<'_, '_>,
    (carga_liquida_propria, carga_liquida_propria_id): (Peso, ElementId),
    (carga_liquida_terceiros, carga_liquida_terceiros_id): (Peso, ElementId),
    (carga_liquida, carga_liquida_id): (Peso, ElementId),
) {
    if carga_liquida_propria.checked_add(carga_liquida_terceiros) != Some(carga_liquida) {
        context.erro_nos_valores(
            "os valores de carga líquida informados não batem",
            regras::PARCELAS_PESO_LIQUIDO,
            [
                (
                    carga_liquida_propria_id,
                    "carga líquida própria definida aqui",
                ),
                (
                    carga_liquida_terceiros_id,
                    "carga líquida de terceiros definida aqui",
                ),
                (carga_liquida_id, "carga líquida total definida aqui"),
            ],
        );
    }
}

/// Exige campos da mercadoria somente quando o estado cheio foi declarado.
fn validar_campos_carga_cheia(
    context: &mut ValidationContext<'_, '_>,
    source_map: &CargaConteinerizadaSourceMap,
) {
    for (nome, elemento) in [
        (
            "CodigoCargaConteinerizada",
            source_map.codigo_carga_conteinerizada,
        ),
        ("PesoCargaLiquida", source_map.peso_carga_liquida),
        ("ImdgCode", source_map.imdg_code),
    ] {
        if elemento.is_none() {
            context.erro_antes_tags_fechamento(
                format!("o contêiner cheio exige <{nome}>"),
                regras::CAMPOS_DETALHAMENTO_CONTEINER,
                [(
                    source_map.element,
                    format!("informe <{nome}> para esta mercadoria"),
                )],
            );
        }
    }
}

/// Agrupa campos de mercadoria incompatíveis com um contêiner declarado vazio.
///
/// Os filhos são violações simultâneas; não representam hipóteses concorrentes.
fn validar_campos_carga_vazia(
    context: &mut ValidationContext<'_, '_>,
    source_map: &CargaConteinerizadaSourceMap,
) {
    context.agrupar(
        "o detalhamento de contêiner vazio deve conter somente <IDConteiner>",
        regras::CAMPOS_DETALHAMENTO_CONTEINER,
        [RotuloDiagnostico::evidencia_na_tag_abertura(
            source_map.element,
        )],
        |grupo| {
            for (nome, elemento) in [
                (
                    "CodigoCargaConteinerizada",
                    source_map.codigo_carga_conteinerizada,
                ),
                ("CEMercante", source_map.ce_mercante),
                ("PesoCargaLiquida", source_map.peso_carga_liquida),
                (
                    "PesoCargaLiquidaPropria",
                    source_map.peso_carga_liquida_propria,
                ),
                (
                    "PesoCargaLiquidaTerceiros",
                    source_map.peso_carga_liquida_terceiros,
                ),
                ("ImdgCode", source_map.imdg_code),
            ] {
                if let Some(elemento) = elemento {
                    grupo.erro_nas_tags_abertura(
                        format!("<{nome}> não se aplica ao contêiner vazio"),
                        regras::CAMPOS_DETALHAMENTO_CONTEINER,
                        [(elemento, "remova o campo se o estado vazio estiver correto")],
                    );
                }
            }
        },
    );
}

/// Testes privados opcionais, mantidos somente na instalação local.
#[cfg(all(test, feature = "local-tests"))]
mod testes_locais {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/local/validation/carga_conteinerizada.rs"
    ));
}
