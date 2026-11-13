//! Modelos XML de uma atracação e de suas estruturas auxiliares.
//!
//! O lifetime `'xml` marca empréstimos do texto original. Campos repetitivos
//! usam `TextoId`, resolvido pelo interner fornecido ao reader e ao validador.

use chrono::NaiveDateTime;

use crate::{
    interner::TextoId,
    xml::models::{
        carga::Carga,
        enums::{NacionalidadeArmador, NavegacaoAtracacao, TipoOperacao},
    },
};

/// Dados de uma ocorrência `<Atracacao>` do documento SDP.
///
/// Os textos são fatias do XML original ou IDs internados. Regras entre campos
/// pertencem à validação, incluindo cronologia, paralisações e coleta.
/// A presença de ambos os identificadores de instalação é representável para
/// permitir um diagnóstico conjunto: as descrições dos campos expressam sua
/// finalidade, não uma garantia de validação. Datas não carregam o offset:
/// o reader preserva horários sem fuso e normaliza datas com fuso para UTC.
/// Uma lista ausente de descontos é representada por `None`.
#[derive(Debug)]
pub struct Atracacao<'xml> {
    /// Identificação do porto organizado informante. É mutuamente exclusiva
    /// com [`Self::codigo_tup`].
    pub trigrama_porto: Option<TextoId>,
    /// Identificação do terminal de uso privado informante. É mutuamente
    /// exclusiva com [`Self::trigrama_porto`].
    pub codigo_tup: Option<TextoId>,
    /// Número da atracação atribuído pela instalação portuária.
    pub numero_atracacao: TextoId,
    /// Número da escala da embarcação, proveniente da tag `<NREscala>`.
    pub nr_escala: Option<&'xml str>,
    /// Código do berço ou local em que ocorreu a atracação.
    pub local_atracacao: TextoId,
    /// Número IMO da embarcação, ou `"0"` quando ele não tiver sido informado.
    ///
    /// Diferentemente de [`Self::numero_inscricao_capitania`], a tag `<IMON>`
    /// é estruturalmente obrigatória no XML.
    pub imon: &'xml str,
    /// Número de inscrição da embarcação na Capitania dos Portos, quando
    /// informado. A ausência da informação é representada pela ausência da
    /// própria tag.
    pub numero_inscricao_capitania: Option<&'xml str>,
    /// Modalidade de navegação realizada pela embarcação.
    pub navegacao: NavegacaoAtracacao,
    /// Natureza operacional da atracação.
    pub tipo_operacao: TipoOperacao,
    /// CNPJ do agente marítimo responsável.
    pub cnpj_agente: TextoId,
    /// Código de nacionalidade do armador definido pelo SDP.
    pub nacionalidade_armador: NacionalidadeArmador,
    /// CNPJ ou código de identificação do armador.
    pub cnpj_codigo_armador: TextoId,
    /// Data e hora de chegada da embarcação à área portuária.
    pub data_hora_chegada: NaiveDateTime,
    /// Data e hora previstas para a atracação.
    pub data_hora_prevista_atracacao: NaiveDateTime,
    /// Data e hora efetivas da atracação.
    pub data_hora_atracacao: NaiveDateTime,
    /// Data e hora efetivas da desatracação.
    pub data_hora_desatracacao: NaiveDateTime,
    /// Início da operação, cuja presença depende do tipo de operação e das cargas.
    pub data_hora_inicio_operacao: Option<NaiveDateTime>,
    /// Término da operação, cuja presença depende do tipo de operação e das cargas.
    pub data_hora_termino_operacao: Option<NaiveDateTime>,
    /// Ocorrências `<TemposDescontos>` descontadas do tempo operacional.
    pub tempos_descontos: Option<Vec<TemposDescontos>>,
    /// Passageiros embarcados, quando o campo é aplicável.
    pub quantidade_passageiros_embarcados: Option<u32>,
    /// Passageiros desembarcados, quando o campo é aplicável.
    pub quantidade_passageiros_desembarcados: Option<u32>,
    /// Passageiros que permaneceram em trânsito, quando o campo é aplicável.
    pub quantidade_passageiros_transito: Option<u32>,
    /// Indica se houve coleta de resíduos da embarcação.
    pub flag_possui_coleta_residuo: bool,
    /// Certificados de coleta de resíduos; presença vinculada à flag.
    pub coletas_residuos: Vec<super::residuos::ColetaResiduo<'xml>>,
    /// Cargas movimentadas durante a atracação.
    pub cargas: Vec<Carga<'xml>>,
}

/// Uma ocorrência `<TemposDescontos>` do XML.
///
/// Armazena o código e os limites informados, sem garantir que a duração seja
/// positiva ou esteja contida na estadia. Essas condições são verificadas em
/// `validation::tempos_descontos` depois da decodificação.
#[derive(Debug)]
pub struct TemposDescontos {
    /// Código SDP do motivo do desconto.
    pub codigo: u32,
    /// Início do intervalo descontado.
    pub data_hora_inicio: NaiveDateTime,
    /// Término do intervalo descontado.
    pub data_hora_termino: NaiveDateTime,
}
