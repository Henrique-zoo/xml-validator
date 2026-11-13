//! Regras semânticas de uma ocorrência `<TemposDescontos>`.
//!
//! O modelo XML usa o nome histórico `TemposDescontos`, enquanto o *Manual do
//! Usuário Externo do sistema SDP*, 5ª edição, versão 1.0, dezembro de 2025,
//! descreve a estrutura como “Paralisações” na seção 2.2.3 (p. 12).
//!
//! A passagem valida cada intervalo e depois soma somente durações positivas.
//! Uma cronologia inválida não participa da comparação de totais.
//! O enunciado geral fica em [`crate::validation::regras_de_negocio::pendentes::TEMPOS_DESCONTOS`];
//! as verificações são chamadas pela passagem pública de validação.
//!
//! # Checklist normativo
//!
//! - [x] tratar a lista de paralisações como facultativa e aceitar uma ou mais
//!   ocorrências quando ela estiver presente;
//! - [x] aceitar os códigos explicitamente descritos 1, 2 e 4 a 9, rejeitar 0
//!   e 3 e aceitar estruturalmente qualquer código a partir de 10;
//! - [x] garantir `DataHoraInicio < DataHoraTermino`;
//! - [x] garantir que início e término estejam contidos no período entre
//!   `DataHoraChegada` e `DataHoraDesatracacao` da atracação;
//! - [x] garantir que a duração total das paralisações seja menor que a duração
//!   total da operação.
//!
//! O Manual não manda rejeitar duplicidades, sobreposições nem ordem não
//! cronológica. Essas regras foram removidas do checklist até que outra fonte
//! normativa as sustente.
//!
//! O Manual remete os códigos a partir de 10 a uma relação que pode mudar.
//! Como o validador não faz I/O externo, a regra prevista os aceita como valores reservados
//! sem afirmar que cada número esteja cadastrado na versão corrente do SDP.

use super::{
    context::{RotuloDiagnostico, ValidationContext},
    regras_de_negocio::pendentes::TEMPOS_DESCONTOS,
};
use crate::xml::{
    models::atracacao::Atracacao,
    reader::{AtracacaoSourceMap, TemposDescontosSourceMap},
};

/// Valida códigos, cronologia, limites da estadia e duração acumulada.
///
/// Não rejeita sobreposições ou duplicidades. Com datas operacionais ausentes
/// ou invertidas, executa as regras individuais e omite a comparação de totais.
pub(super) fn validar_tempos_descontos(
    context: &mut ValidationContext<'_, '_>,
    atracacao: &Atracacao<'_>,
    mapa: &AtracacaoSourceMap,
    mapas: &[TemposDescontosSourceMap],
) {
    let descontos = atracacao.tempos_descontos.as_deref().unwrap_or(&[]);
    assert_eq!(
        descontos.len(),
        mapas.len(),
        "descontos e mapas devem estar alinhados"
    );
    let mut duracao = 0i128;
    let mut intervalos_validos = true;
    for (desconto, m) in descontos.iter().zip(mapas) {
        if matches!(desconto.codigo, 0 | 3) {
            context.erro_nos_valores(
                "código de paralisação não permitido",
                TEMPOS_DESCONTOS,
                [(m.codigo, "informe 1, 2 ou um código a partir de 4")],
            );
        }
        if desconto.data_hora_inicio >= desconto.data_hora_termino {
            intervalos_validos = false;
            context.erro_nos_valores(
                "o início da paralisação deve anteceder o término",
                TEMPOS_DESCONTOS,
                [
                    (m.data_hora_inicio, "início informado"),
                    (m.data_hora_termino, "término informado"),
                ],
            );
        } else {
            duracao += nanossegundos(desconto.data_hora_termino - desconto.data_hora_inicio);
        }
        for (data, id) in [
            (desconto.data_hora_inicio, m.data_hora_inicio),
            (desconto.data_hora_termino, m.data_hora_termino),
        ] {
            if data < atracacao.data_hora_chegada || data > atracacao.data_hora_desatracacao {
                context.erro_nos_valores(
                    "a paralisação deve estar contida na estadia da embarcação",
                    TEMPOS_DESCONTOS,
                    [
                        (id, "data fora da estadia"),
                        (mapa.data_hora_chegada, "limite inicial"),
                        (mapa.data_hora_desatracacao, "limite final"),
                    ],
                );
            }
        }
    }
    if intervalos_validos
        && !descontos.is_empty()
        && let (Some(inicio), Some(termino), Some(inicio_id), Some(termino_id)) = (
            atracacao.data_hora_inicio_operacao,
            atracacao.data_hora_termino_operacao,
            mapa.data_hora_inicio_operacao,
            mapa.data_hora_termino_operacao,
        )
        && termino > inicio
        && duracao >= nanossegundos(termino - inicio)
    {
        let mut labels = vec![
            RotuloDiagnostico::no_valor(inicio_id, "início da operação"),
            RotuloDiagnostico::no_valor(termino_id, "término da operação"),
        ];
        labels.extend(mapas.iter().take(3).map(|m| {
            RotuloDiagnostico::na_tag_abertura(m.element, "paralisação incluída no total")
        }));
        context.erro_com_detalhes(
            "a duração total das paralisações deve ser menor que a duração da operação",
            TEMPOS_DESCONTOS,
            labels,
        );
    }
}

/// Converte a duração em nanossegundos sem o limite de `TimeDelta::num_nanoseconds`.
fn nanossegundos(duracao: chrono::TimeDelta) -> i128 {
    i128::from(duracao.num_seconds()) * 1_000_000_000 + i128::from(duracao.subsec_nanos())
}

/// Testes privados opcionais, mantidos somente na instalação local.
#[cfg(all(test, feature = "local-tests"))]
mod testes_locais {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/local/validation/tempos_descontos.rs"
    ));
}
