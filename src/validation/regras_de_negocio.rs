//! Enunciados gerais das regras de negócio usadas pela validação.
//!
//! Cada constante descreve uma regra independentemente do XML em análise.
//! Os validadores escolhem o enunciado para `ValidationError::details`; os
//! valores encontrados, as evidências e as sugestões ficam na mensagem e nas
//! labels. Os textos são estáticos e compartilhados, sem alocação por erro.
//! Uma interface pode consultá-los sem executar o reader ou o validador.
//!
//! Este módulo centraliza a descrição, não a execução: não consulta arquivos,
//! não contém predicados e não habilita regras automaticamente. Os checklists
//! dos módulos de entidade continuam registrando o que está conectado,
//! parcialmente implementado ou pendente. Em particular, um enunciado completo
//! não significa que todas as suas condições já sejam verificadas.
//!
//! # Fontes e revisões
//!
//! As referências `Manual` indicam o *Manual do Usuário Externo do sistema
//! SDP*, 5ª edição, versão 1.0, dezembro de 2025, pelas páginas impressas.
//! As adaptações locais são identificadas na documentação de cada constante:
//! não devem ser confundidas com exigências explícitas do Manual. Nenhuma regra
//! local comprova a existência ou a situação de uma entidade em cadastro externo.
//! O caminho histórico `pendentes` é preservado como alias de compatibilidade;
//! seus enunciados também estão disponíveis diretamente neste módulo.

/// Identificação do informante conforme o leiaute: Manual §2.2.1, p. 9, e XSDs locais.
pub const IDENTIFICADOR_INSTALACAO: &str = "Cada atracação deve identificar a instalação informante por um único campo: \
     <TrigramaPorto> no leiaute de porto organizado (SDPv2.xsd), ou <CodigoTUP> \
     no leiaute de instalação autorizada, como terminal de uso privado (SDPTup.xsd). \
     Os dois campos são mutuamente exclusivos.";

/// Formato do porto informante: Manual §2.2.1, p. 9; ASCII maiúsculo é a convenção local.
///
/// Não estender esta exigência aos portos de origem/destino: o Manual §2.2.4,
/// p. 16, também exemplifica códigos com algarismos para esses locais.
pub const TRIGRAMA_PORTO: &str = "O identificador do porto informante em <TrigramaPorto> deve conter exatamente \
     três letras ASCII maiúsculas, de A a Z.";

/// Formato do identificador de instalação autorizada: Manual §2.2.1, p. 9.
/// A validação local verifica o formato da UF, não sua existência.
pub const CODIGO_TUP: &str = "O identificador <CodigoTUP> deve conter o prefixo BR, duas letras ASCII \
     maiúsculas para a UF e três algarismos ASCII, totalizando sete caracteres \
     no padrão BRLLNNN, como BRCE001.";

/// Identificador da atracação: Manual §2.2.1, p. 9; ASCII é a convenção local.
pub const NUMERO_ATRACACAO: &str =
    "O valor de <NumeroAtracacao> deve ter no máximo 15 caracteres alfanuméricos ASCII.";

/// Identificador do local de atracação: Manual §2.2.1, p. 10.
pub const LOCAL_ATRACACAO: &str =
    "O valor de <LocalAtracacao> deve conter exatamente quatro caracteres alfanuméricos ASCII.";

/// Identificação alternativa da embarcação: Manual §2.2.1, p. 10, e §3.1, p. 19.
pub const IDENTIFICACAO_EMBARCACAO: &str = "A embarcação deve possuir ao menos um identificador: um número IMO diferente \
     de zero ou um número de inscrição na Capitania. Os dois podem ser informados juntos.";

/// Número IMO e sentinela de ausência: Manual §2.2.1, p. 10.
pub const NUMERO_IMO: &str = "A tag <IMON> deve conter um número IMO de sete algarismos ASCII. Quando não \
     houver número IMO, a tag deve ser mantida com o valor exato 0; outras sequências \
     compostas somente por zeros não representam a ausência do número.";

/// Número de inscrição e omissão da tag: Manual §2.2.1, p. 10.
pub const NUMERO_INSCRICAO_CAPITANIA: &str = "A tag <NumeroInscricaoCapitania>, quando presente, deve conter exatamente \
     dez algarismos ASCII. Quando o número não for informado, a tag deve ser omitida.";

/// Formação local de CNPJs: Manual §2.2.1, p. 10, e §2.2.4, p. 13, com adaptação.
///
/// O helper atual aceita formas compacta e pontuada, numérica e alfanumérica.
/// O agente e as empresas de coleta exigem a representação compacta de 14 posições.
pub const FORMACAO_CNPJ: &str = "Os campos destinados a CNPJ devem conter um identificador estruturalmente \
     válido, com comprimento, caracteres, separadores e dígitos verificadores \
     compatíveis. A validação local aceita 14 caracteres sem pontuação ou 18 na \
     forma XX.XXX.XXX/XXXX-DD, sem consultar existência ou situação cadastral.";

/// Identificação do armador: Manual §2.2.1, pp. 10–11, e §3.1, pp. 19–20.
/// A rejeição de CNPJ válido como código estrangeiro é a interpretação local.
pub const IDENTIFICACAO_ARMADOR: &str = "Para <NacionalidadeArmador> igual a 1 (brasileiro), <CnpjCodigoArmador> deve \
     conter um CNPJ estruturalmente válido. Para nacionalidade 2 (estrangeiro), \
     deve conter o código do armador estrangeiro, não um CNPJ válido. A validação \
     local não consulta o cadastro de armadores.";

/// Passageiros: Manual §2.2.1, p. 11, com positividade exigida pelo projeto.
pub const QUANTIDADES_PASSAGEIROS: &str = "As tags <QuantidadePassageirosEmbarcados>, <QuantidadePassageirosDesembarcados> \
     e <QuantidadePassageirosTransito> são permitidas somente nas operações de \
     passageiros (2) ou mistas (7). Nessas operações, ao menos uma quantidade positiva \
     deve ser informada; não é necessário informar as três. Nas demais operações, \
     essas tags devem ser omitidas, inclusive quando seu valor seria zero.";

/// Ordem dos eventos; a revisão local exige desigualdade estrita após a atracação.
pub const CRONOLOGIA_ATRACACAO: &str = pendentes::CRONOLOGIA_ESTRITA;

/// Presença de datas e cargas: Manual §2.2.2, p. 12, e §3.1–3.2, p. 20.
/// O enunciado é compartilhado pelos diagnósticos de datas e de carga ausente.
pub const DATAS_E_CARGAS_POR_OPERACAO: &str = "Atracações de movimentação de carga (1) ou mistas (7) devem informar ao menos \
     uma <Carga> e as datas de início e término da operação. Em apoio (3) e \
     abastecimento (5), essas datas devem ser informadas quando houver cargas \
     movimentadas; informar as datas também exige o registro das cargas.";

/// Seleção da representação: estratégia local baseada no Manual §2.2.4, p. 13.
pub const REPRESENTACAO_CARGA: &str = "A presença de <CheioVazio>, de <CargaConteinerizada> ou de um código ISO \
     reconhecido declara conteinerização. Essa representação exige código ISO, \
     estado cheio/vazio e detalhamento. Sem esses indicadores, os códigos de \
     apoio são tratados separadamente; as demais mercadorias usam quatro \
     algarismos NCM/SH4. Um código não reconhecido, sem indicadores e fora de \
     apoio, requer esclarecer a representação pretendida.";

/// Código de contêiner: Manual §2.2.4, p. 13; consulta ao CSV incorporado pelo projeto.
pub const CODIGO_CONTEINER: &str = "A representação de conteinerização exige em <CodigoCarga> um código ISO de \
     quatro caracteres alfanuméricos presente no catálogo incorporado ao programa. \
     O formato alfanumérico, sozinho, não comprova que o código identifique um contêiner.";

/// Código de mercadoria: Manual §2.2.4, p. 13, sem consulta a cadastro.
pub const CODIGO_MERCADORIA: &str = "Mercadorias não conteinerizadas e fora de operação de apoio devem ser \
     identificadas em <CodigoCarga> pelos quatro primeiros algarismos ASCII do \
     NCM (NCM/SH4). Os códigos próprios de apoio não são validados como NCM/SH4.";

/// Tag NCM/SH8: revisão do leiaute adotada no projeto, posterior ao Manual utilizado.
pub const CODIGO_NCM: &str = "A tag opcional <CodigoNCM> representa o NCM/SH8 e, quando presente, deve conter \
     exatamente oito algarismos ASCII. A validação do formato não comprova a \
     existência do código em cadastro oficial.";

/// Estado do contêiner: Manual §2.2.4, p. 13; conversão C/V feita pelo reader.
pub const ESTADO_CONTEINER: &str = "Cargas referentes a contêiner devem informar <CheioVazio> com C (cheio) ou \
     V (vazio). Essa tag deve ser omitida em cargas não conteinerizadas.";

/// Detalhamento dos contêineres: Manual §2.2.6, pp. 18–19, e §3.2, p. 20.
pub const DETALHAMENTO_CONTEINER: &str = "Cargas conteinerizadas devem possuir blocos <CargaConteinerizada> que \
     identifiquem cada contêiner e, quando cheio, cada mercadoria contida nele. \
     Contêineres vazios também devem ser identificados no detalhamento.";

/// Natureza dos contêineres: Manual §2.2.4, p. 13.
pub const NATUREZA_CONTEINER: &str = "Cargas referentes a contêiner, cheio ou vazio, devem informar <NaturezaCarga> \
     igual a 3 (carga geral).";

/// Relação entre sentido e operação adotada pelo projeto.
///
/// O Manual §2.2.4, pp. 13–14, menciona explicitamente zero em abastecimento.
/// A omissão e a extensão a safamento/remoção pertencem à revisão local, não
/// a uma transcrição do Manual. A regra cobre todo o conjunto de operações.
pub const SENTIDO_POR_OPERACAO: &str = "A tag <Sentido> é obrigatória e deve conter 1 (desembarque) ou 2 (embarque), \
     exceto nas operações de abastecimento (5), safamento (8) e remoção a bordo (10). \
     Nessas três operações, a tag deve ser omitida ou informada com valor 0.";

/// Navegação e omissão conjunta: Manual §2.2.4–2.2.5, pp. 14 e 18.
///
/// O projeto segue a omissão textual da p. 18. A p. 14 também descreve zero
/// para algumas operações. Essa alternativa não foi adotada no contrato local.
pub const NAVEGACAO_ORIGEM_DESTINO: &str = "Nas operações de abastecimento (5), safamento (8), remoção a bordo (10), \
     operação intermediária (11) e transferência interna (12), deve-se omitir \
     navegação, origem e destino, incluindo suas tags de país, porto, TUP e área. \
     Nas demais operações, <Navegacao> deve ser informada com um código de 1 a 5.";

/// Representações alternativas de origem/destino: Manual §2.2.4, pp. 14–16.
pub const REPRESENTACAO_INSTALACAO: &str = "Quando aplicáveis, origem e destino devem identificar cada instalação por \
     uma única representação: porto pela dupla <BigramaPaisOrigem>/<TrigramaPortoOrigem> \
     ou <BigramaPaisDestino>/<TrigramaPortoDestino>; instalação privada nacional \
     por <CodigoTUPOrigem> ou <CodigoTUPDestino>. Código TUP e identificadores \
     de porto não devem coexistir na mesma extremidade.";

/// Correspondência ao informante: Manual §2.2.4, p. 16.
/// Para porto informante brasileiro, o país correspondente deve ser BR.
pub const INSTALACAO_POR_SENTIDO: &str = "A instalação informante deve ser a origem da carga em embarque (2) e o \
     destino em desembarque (1). Nessa extremidade, a representação e o \
     identificador devem corresponder aos da instalação declarada na atracação: \
     país e trigrama para porto, ou código TUP para instalação privada nacional.";

/// Compatibilidade entre operações: Manual §2.2.5, p. 18.
pub const OPERACAO_CARGA_ATRACACAO: &str = "Os tipos principais de operação de carga (13 a 20) são permitidos somente \
     em atracações de movimentação de carga (1) ou mistas (7). Em atracações de \
     apoio (3) ou abastecimento (5), o tipo de operação da carga deve ser o de \
     mesmo nome e código da operação da atracação.";

/// Restrições de safamento/remoção: Manual §2.2.5, p. 18.
/// Contêineres usam o catálogo ISO; a quantidade é obrigatória nas duas representações.
pub const SAFAMENTO_REMOCAO: &str = "Nas operações de safamento (8) e remoção a bordo (10), são permitidos códigos \
     de carga nos intervalos 8701–8716, 8401–8429 e 4401–4421, além dos códigos \
     de contêiner. Devem ser informadas a natureza, a quantidade e o peso total \
     da carga em toneladas.";

/// CE opcional de quinze caracteres: revisão de leiaute registrada pelo projeto.
/// O Manual §2.2.4 e §2.2.6 ainda descrevia exigência por navegação.
pub const CE_MERCANTE: &str = "<CEMercante> é opcional na revisão local do leiaute. Quando informado, \
     deve conter quinze caracteres. Para contêiner vazio, deve ser omitido \
     tanto na <Carga> quanto em seus detalhamentos.";

/// Quantidade de carga geral: Manual §2.2.4, p. 15.
pub const QUANTIDADE_CARGA_GERAL: &str = "Cargas com <NaturezaCarga> igual a 3 (carga geral) devem informar <Quantidade> \
     em unidades, incluindo mercadorias embaladas, unitizadas e cargas gerais soltas.";

/// Limites de quantidade: Manual §2.2.4, p. 15.
pub const LIMITES_QUANTIDADE: &str = "A quantidade de contêineres por atracação não pode exceder 32.767 unidades. \
     As demais cargas admitem no máximo 99.999.999 unidades.";

/// Limite superior já adotado pelo projeto, sem fonte localizada no Manual.
/// A verificação atual se aplica somente à representação conteinerizada.
pub const LIMITE_PESO_CONTEINERIZADO: &str = "Na validação local, <PesoCargaBruta> de uma carga conteinerizada não pode \
     exceder 564.650 toneladas.";

/// Soma líquida e estado: regra relacional do projeto baseada em cheio/vazio.
pub const PESO_LIQUIDO_POR_ESTADO: &str = "A soma dos pesos líquidos de todos os detalhamentos da carga deve ser \
     positiva quando <CheioVazio> for C (cheio) e igual a 0.000 tonelada quando \
     for V (vazio). Um peso ausente não deve ser interpretado como peso zero.";

/// Peso bruto e líquido: Manual §2.2.4 e §2.2.6, pp. 15 e 19; relação local.
/// A revisão local exige tara positiva na comparação com o peso líquido.
pub const PESO_BRUTO_E_LIQUIDO: &str = "O peso bruto de uma carga conteinerizada inclui as mercadorias e a tara dos \
     contêineres. Por isso, <PesoCargaBruta> deve ser maior que a soma dos \
     valores de <PesoCargaLiquida> de todos os seus detalhamentos.";

/// Limite de contêiner vazio: decisão explícita do projeto, não atribuída ao Manual.
pub const LIMITE_PESO_CONTEINER_VAZIO: &str = "Uma carga marcada como vazia em <CheioVazio> pode possuir peso bruto de no \
     máximo 20.000 toneladas (vinte toneladas), independentemente da quantidade \
     de contêineres ou de blocos de detalhamento.";

/// Limite técnico da representação exata de pesos, não regra normativa do SDP.
pub const REPRESENTACAO_SOMA_PESOS: &str = "Os pesos são representados em quilogramas inteiros sem perda de precisão. \
     A soma deve caber em um inteiro sem sinal de 64 bits; um estouro não pode \
     ser interpretado como um peso válido nem usado nas comparações de massa.";

/// Identificador de contêiner: Manual §2.2.6, p. 19; sem dígito verificador adicional.
pub const ID_CONTEINER: &str = "A tag <IDConteiner> deve conter exatamente onze caracteres alfanuméricos \
     ASCII, tanto para contêiner cheio quanto para vazio.";

/// Código da mercadoria no detalhamento: Manual §2.2.6, p. 18.
pub const CODIGO_CARGA_CONTEINERIZADA: &str = "A tag <CodigoCargaConteinerizada> identifica a mercadoria dentro do contêiner \
     pelos quatro primeiros algarismos ASCII do NCM (NCM/SH4), não pelo código \
     ISO do contêiner. Quando presente, deve conter exatamente quatro algarismos.";

/// Campos por estado: Manual §2.2.6, pp. 18–19, §10, pp. 43–45, e §15.4–15.5, pp. 52–53.
/// A opcionalidade do CE no cheio segue a revisão local descrita em [`CE_MERCANTE`].
pub const CAMPOS_DETALHAMENTO_CONTEINER: &str = "Cada mercadoria em contêiner cheio deve informar <CodigoCargaConteinerizada>, \
     <PesoCargaLiquida> e <ImdgCode>, usando 0 no IMDG para produto não perigoso. \
     Para contêiner vazio, o detalhamento deve conter somente <IDConteiner>, \
     sem código de mercadoria, CE-Mercante, pesos líquidos ou IMDG.";

/// Igualdade das parcelas líquidas: regra adicional do projeto, não declarada no Manual.
pub const PARCELAS_PESO_LIQUIDO: &str = "Quando <PesoCargaLiquidaPropria>, <PesoCargaLiquidaTerceiros> e <PesoCargaLiquida> \
     estiverem presentes no detalhamento de contêiner cheio, a soma das parcelas \
     própria e de terceiros deve ser igual ao peso líquido total. A soma não \
     pode exceder o limite da representação numérica.";

/// Caminho histórico dos enunciados agora conectados, preservado para consumidores.
pub mod pendentes {
    /// Unicidade local: recorte verificável do Manual §2.2.1, pp. 9–10.
    pub const UNICIDADE_ATRACACAO: &str = "No mesmo documento, não devem existir números de atracação repetidos \
         para o mesmo ano e local de atracação. Esta verificação não consulta \
         atracações enviadas anteriormente ao SDP.";

    /// Consolidação: Manual §2.1, p. 9, e §3.2, p. 20.
    pub const CONSOLIDACAO_CARGAS: &str = "Cargas de uma mesma atracação com características idênticas, exceto o peso, \
         devem ser consolidadas em um registro com a soma dos pesos. Registros \
         separados são admitidos quando houver diferença de origem ou destino.";

    /// Máscara do agente: Manual §2.2.1, p. 10.
    pub const CNPJ_AGENTE_SEM_MASCARA: &str = "O CNPJ do agente marítimo deve ser informado em <CnpjAgente> com 14 \
         caracteres e sem máscara.";

    /// Revisão de cronologia anotada no projeto, mais estrita que o Manual §2.2.2.
    pub const CRONOLOGIA_ESTRITA: &str = "As datas devem respeitar chegada <= atracação < início da operação < \
         término da operação < desatracação.";

    /// Áreas do informante: Manual §2.2.4, pp. 14–15.
    pub const AREA_POR_SENTIDO: &str = "Quando a identificação das áreas for aplicável, embarque (2) exige \
         <CodigoAreaOrigem> e desembarque (1) exige <CodigoAreaDestino> da \
         instalação informante. A operação direta, sem armazenagem, usa o código 9999.";

    /// Formato e positividade do peso: Manual §2.2.4 e §2.2.6, pp. 15 e 19.
    /// A conversão exata de toneladas e até três decimais já pertence ao reader.
    pub const FORMATO_PESOS: &str = "Os pesos são informados em toneladas, com até três casas decimais e \
         representação de até 13 posições. <PesoCargaBruta> deve ser maior que \
         zero e, para contêineres, inclui a carga e a tara.";

    /// Presença de IMDG: Manual §10, pp. 43–45; sem consulta de cadastro.
    pub const IMDG_CARGA: &str = "Toda carga e cada mercadoria dentro de um contêiner devem informar \
         <ImdgCode>. Produtos não perigosos devem ser declarados com o valor 0.";

    /// Paralisações: Manual §2.2.3, p. 12, com tratamento local dos códigos reservados.
    pub const TEMPOS_DESCONTOS: &str = "O informe de paralisações é facultativo e admite uma ou mais ocorrências. \
         Os códigos admitidos localmente são 1, 2 e 4 a 9, além de valores a partir \
         de 10, aceitos como reservados sem comprovação de cadastro. Cada início \
         deve anteceder o término, ambos entre chegada e desatracação. A duração \
         total das paralisações deve ser menor que a duração total da operação.";

    /// Ocorrência de coleta: Manual §2.2.1, p. 11, e §9.2.1, pp. 40–42.
    pub const COLETA_RESIDUOS: &str = "<FlagPossuiColetaResiduo> deve conter 0 ou 1, independentemente do tipo \
         de operação da atracação. O valor 1 exige ao menos um bloco completo \
         <ColetaResiduo>; o valor 0 proíbe esses blocos. Cada coleta deve conter \
         certificado, CNPJs de coletor e receptor, datas de início, término e \
         entrega e ao menos um <Residuo> com tipo, quantidades e unidade.";
}

pub use pendentes::{
    AREA_POR_SENTIDO, CNPJ_AGENTE_SEM_MASCARA, COLETA_RESIDUOS, CONSOLIDACAO_CARGAS,
    CRONOLOGIA_ESTRITA, FORMATO_PESOS, IMDG_CARGA, TEMPOS_DESCONTOS, UNICIDADE_ATRACACAO,
};
