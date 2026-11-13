use std::{
    collections::{
        HashMap,
        hash_map::{Entry as HashMapEntry, VacantEntry},
    },
    num::NonZeroU32,
};

/// Identificador de uma string registrada em um [`Interner`].
///
/// Cada string distinta recebe um identificador sequencial, começando em `1`,
/// na ordem de sua primeira inserção. Consultas ou inserções posteriores do
/// mesmo conteúdo reutilizam o identificador existente.
///
/// O identificador corresponde ao índice da string no vetor interno do
/// interner, acrescido de `1`. O valor zero é inválido e não pode ser
/// representado porque o tipo utiliza [`NonZeroU32`].
///
/// # Invariantes
///
/// - Identificadores produzidos pelo interner são sempre diferentes de zero.
/// - Strings iguais recebem o mesmo identificador.
/// - Strings distintas recebem identificadores distintos.
/// - Um identificador não muda após sua atribuição.
///
/// # Escopo
///
/// Identificadores só podem ser comparados e resolvidos dentro do interner
/// que os produziu. A CLI usa uma tabela por arquivo; a biblioteca permite
/// compartilhar uma tabela entre documentos cujos textos permaneçam vivos.
///
/// Os valores também não são identificadores persistentes entre execuções:
/// a atribuição depende da ordem em que as strings forem inseridas.
///
/// # Representação
///
/// A ordem compara a posição de internação, não o conteúdo lexical dos textos.
/// Assim como a igualdade, só tem significado dentro do mesmo interner.
/// `#[repr(transparent)]` garante a mesma representação de memória de
/// [`NonZeroU32`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextoId(NonZeroU32);

impl TextoId {
    /// Converte o identificador para o índice correspondente no vetor interno.
    ///
    /// Como os identificadores começam em `1` e o vetor é indexado a partir
    /// de `0`, o índice é calculado por `id - 1`.
    ///
    /// # Retorno
    ///
    /// Índice baseado em zero, representado por `usize`.
    ///
    /// # Observações
    ///
    /// A conversão não verifica se o índice está presente em algum interner.
    /// Essa validação ocorre implicitamente quando [`Interner::resolve`]
    /// indexa o vetor. O método não realiza alocações e é avaliável em
    /// contexto constante.
    const fn index(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

/// Resultado de uma consulta à tabela de strings de um [`Interner`].
///
/// Representa uma string já internada ou uma entrada vazia, que pode ser
/// preenchida sem repetir a busca no `HashMap`.
///
/// A consulta é efetuada por [`Interner::entry`]. Para obter o identificador
/// da string e inseri-la somente quando necessário, use [`Entry::or_insert`].
///
/// # Lifetimes
///
/// - `'entry`: duração do empréstimo mutável dos campos do interner retidos
///   pela entrada vazia.
/// - `'xml`: validade das referências às strings que podem ser armazenadas.
///
/// A entrada pode ter duração menor que as strings referenciadas; em
/// particular, `'xml` deve durar pelo menos tanto quanto `'entry`.
///
/// # Invariantes
///
/// Os métodos [`Interner::entry`] e [`Entry::or_insert`] preservam a
/// correspondência entre o mapa de identificadores e o vetor de strings.
/// Como os campos de [`Entry::Vacant`] são públicos, modificá-los diretamente
/// pode violar essa correspondência. Prefira [`Entry::or_insert`].
///
/// # Exemplos
///
/// ```rust
/// use xml_transformer::interner::{Entry, Interner};
///
/// let mut interner = Interner::new();
///
/// match interner.entry("nome") {
///     Entry::Occupied(id) => println!("Já internada: {id:?}"),
///     vacant @ Entry::Vacant { .. } => {
///         let id = vacant.or_insert();
///         println!("Nova entrada: {id:?}");
///     }
/// }
/// ```
pub enum Entry<'entry, 'xml> {
    /// String já registrada no interner.
    ///
    /// Contém o [`TextoId`] anteriormente atribuído. Nenhuma inserção é
    /// necessária para utilizar esse identificador.
    Occupied(TextoId),

    /// String ainda não registrada no interner.
    ///
    /// Mantém a entrada vazia do mapa e acesso ao vetor de strings, para
    /// permitir uma inserção posterior sem repetir a busca. Descartar esta
    /// variante sem inseri-la não altera o interner.
    ///
    /// # Atenção
    ///
    /// Os campos são públicos. Inserir diretamente no mapa por meio de
    /// `entry.insert(...)` ou modificar `texto` pode quebrar os invariantes
    /// do interner. Utilize [`Entry::or_insert`] para inserir corretamente.
    Vacant {
        /// Entrada vazia localizada no mapa, incluindo a chave consultada.
        ///
        /// Seu método `insert` utiliza a busca já realizada para registrar
        /// um par chave–identificador no `HashMap`.
        entry: VacantEntry<'entry, &'xml str, TextoId>,

        /// Empréstimo mutável do vetor de strings do interner.
        ///
        /// O vetor registra as chaves na ordem em que seus identificadores
        /// são atribuídos. É emprestado separadamente do `HashMap` para
        /// permitir a atualização das duas estruturas na mesma operação.
        texto: &'entry mut Vec<&'xml str>,
    },
}

/// Tabela de internamento de strings de um documento XML.
///
/// Associa cada conteúdo textual distinto a um [`TextoId`], permitindo
/// representar ocorrências repetidas por um identificador compacto.
///
/// O interner **não copia nem possui os bytes das strings**: armazena apenas
/// referências `&'xml str`. A comparação e a deduplicação são feitas pelo
/// conteúdo, não pelo endereço das referências.
///
/// O projeto utiliza uma única instância compartilhada deste tipo. A API,
/// entretanto, não impõe essa unicidade: [`Interner::new`] cria instâncias
/// independentes quando chamado.
///
/// # Lifetime
///
/// O parâmetro `'xml` representa a validade das referências guardadas no
/// interner. Pode corresponder ao período em que o documento XML e seus
/// trechos de texto permanecem acessíveis. Literais `&'static str` também
/// podem ser armazenados, pois `'static` satisfaz esse requisito.
///
/// # Invariantes
///
/// Para cada par `(s, id)` armazenado em `ids`:
///
/// - `id` é diferente de zero;
/// - `texto[id - 1]` contém uma string igual a `s`;
/// - cada string distinta possui um único identificador;
/// - os identificadores começam em `1` e são atribuídos sequencialmente.
///
/// Esses invariantes são preservados pelos métodos desta implementação.
/// Como [`Entry::Vacant`] expõe campos mutáveis, o código chamador também
/// precisa evitar modificações diretas que os violem.
///
/// # Complexidade
///
/// Para `n` strings distintas, consulta e inserção têm custo esperado `O(1)`
/// em relação a `n`, desconsiderando o custo de hashing e comparação das
/// strings. A inserção também pode exigir alocação e realocação amortizada.
/// As estruturas auxiliares ocupam espaço `O(n)`, sem contar os bytes dos
/// textos referenciados.
///
/// # Exemplos
///
/// ```rust
/// use xml_transformer::interner::Interner;
///
/// let mut interner = Interner::new();
/// let primeiro = interner.entry("nome").or_insert();
/// let segundo = interner.entry("idade").or_insert();
/// let repetido = interner.entry("nome").or_insert();
///
/// assert_eq!(primeiro, repetido);
/// assert_ne!(primeiro, segundo);
/// assert_eq!(interner.resolve(primeiro), "nome");
/// ```
#[derive(Default)]
pub struct Interner<'xml> {
    /// Associa o conteúdo de cada string ao respectivo identificador.
    ///
    /// As chaves referenciam strings válidas por `'xml`. O mapa permite
    /// localizar o identificador sem examinar todo o vetor de strings.
    ids: HashMap<&'xml str, TextoId>,

    /// Armazena as strings na ordem de atribuição dos identificadores.
    ///
    /// Uma string cujo identificador é `id` ocupa a posição `id - 1`.
    /// O vetor mantém somente referências, não cópias dos textos.
    texto: Vec<&'xml str>,
}

impl<'xml> Interner<'xml> {
    /// Cria um interner vazio.
    ///
    /// Inicializa o mapa e o vetor de strings sem registrar entradas.
    /// Nenhum identificador é atribuído durante a construção.
    ///
    /// # Retorno
    ///
    /// Um [`Interner`] vazio, capaz de armazenar referências válidas
    /// pelo lifetime `'xml`.
    ///
    /// # Complexidade
    ///
    /// `O(1)` em tempo e espaço iniciais.
    ///
    /// # Exemplos
    ///
    /// ```rust
    /// use xml_transformer::interner::Interner;
    ///
    /// let mut interner = Interner::new();
    /// let id = interner.entry("elemento").or_insert();
    /// assert_eq!(interner.resolve(id), "elemento");
    /// ```
    pub fn new() -> Self {
        Self {
            ids: HashMap::new(),
            texto: Vec::new(),
        }
    }

    /// Consulta a entrada correspondente ao conteúdo de `s`.
    ///
    /// Executa uma busca no `HashMap` e retorna a entrada ocupada ou um
    /// contexto de inserção. A consulta, isoladamente, não armazena a string.
    ///
    /// # Parâmetros
    ///
    /// - `s`: string consultada. A referência deve ser válida por `'xml`
    ///   porque poderá ser guardada no interner em uma inserção posterior.
    ///
    /// # Retorno
    ///
    /// - [`Entry::Occupied`], com o identificador já associado a `s`;
    /// - [`Entry::Vacant`], com a posição de inserção localizada no mapa e
    ///   acesso ao vetor de strings.
    ///
    /// # Empréstimos
    ///
    /// O retorno está vinculado ao empréstimo mutável de `self`. Enquanto
    /// esse empréstimo estiver ativo, não é possível realizar outro acesso
    /// incompatível ao interner. Esse período pode ser menor que `'xml`.
    ///
    /// # Complexidade
    ///
    /// `O(1)` esperado em relação ao número de entradas, além do custo
    /// de hashing e comparação do conteúdo da string.
    ///
    /// # Exemplos
    ///
    /// ```rust
    /// use xml_transformer::interner::{Entry, Interner};
    ///
    /// let mut interner = Interner::new();
    /// let id = interner.entry("nome").or_insert();
    ///
    /// assert!(matches!(
    ///     interner.entry("nome"),
    ///     Entry::Occupied(found) if found == id
    /// ));
    /// ```
    pub fn entry(&mut self, s: &'xml str) -> Entry<'_, 'xml> {
        match self.ids.entry(s) {
            HashMapEntry::Occupied(entry) => Entry::Occupied(*entry.get()),
            HashMapEntry::Vacant(entry) => Entry::Vacant {
                entry,
                texto: &mut self.texto,
            },
        }
    }

    /// Recupera a string associada a um [`TextoId`].
    ///
    /// Converte o identificador em um índice baseado em zero e acessa o vetor. A operação
    /// não procura no `HashMap` nem aloca ou copia a string.
    ///
    /// # Parâmetros
    ///
    /// - `id`: identificador de uma string já internada **nesta instância**.
    ///
    /// # Retorno
    ///
    /// A referência `&'xml str` originalmente registrada para o
    /// identificador. O lifetime do resultado está associado aos dados do
    /// XML, não à duração do empréstimo `&self` usado nesta chamada.
    ///
    /// # Panics
    ///
    /// Entra em pânico se `id` não corresponder a uma posição existente no
    /// vetor, por exemplo, quando for um identificador obtido de outra
    /// instância que possua mais entradas. O valor não zero, isoladamente,
    /// não garante que o identificador exista neste interner.
    ///
    /// # Complexidade
    ///
    /// `O(1)` em tempo e espaço adicionais.
    ///
    /// # Exemplos
    ///
    /// ```rust
    /// use xml_transformer::interner::Interner;
    ///
    /// let mut interner = Interner::new();
    /// let id = interner.entry("atributo").or_insert();
    ///
    /// assert_eq!(interner.resolve(id), "atributo");
    /// ```
    pub fn resolve(&self, id: TextoId) -> &'xml str {
        self.texto[id.index()]
    }
}

impl<'entry, 'xml> Entry<'entry, 'xml> {
    /// Retorna o identificador da string, inserindo-a caso esteja ausente.
    ///
    /// Se a entrada estiver ocupada, retorna o [`TextoId`] existente sem
    /// modificar o interner. Caso esteja vazia, efetiva a inserção:
    ///
    /// 1. Calcula o próximo identificador a partir do tamanho do vetor.
    /// 2. Recupera a chave mantida pela entrada vazia do `HashMap`.
    /// 3. Acrescenta a referência ao vetor de strings.
    /// 4. Insere o identificador no mapa usando a posição já localizada.
    /// 5. Retorna o novo identificador.
    ///
    /// Não é realizada uma segunda busca no mapa.
    ///
    /// # Retorno
    ///
    /// O identificador existente ou recém-atribuído à string.
    ///
    /// # Consumo da entrada
    ///
    /// O método consome `self`. A entrada não pode ser reutilizada e os
    /// empréstimos retidos por ela deixam de ser necessários após a chamada.
    ///
    /// # Panics
    ///
    /// Entra em pânico se não for possível representar o próximo
    /// identificador em um `u32` não zero, após `u32::MAX` inserções
    /// distintas. Falhas de alocação também podem encerrar a execução,
    /// conforme o comportamento de `Vec` e `HashMap`.
    ///
    /// # Complexidade
    ///
    /// - Entrada ocupada: `O(1)`.
    /// - Entrada vazia: `O(1)` amortizado esperado, desconsiderando o custo
    ///   de alocação. A busca já ocorreu em [`Interner::entry`].
    ///
    /// # Exemplos
    ///
    /// ```rust
    /// use xml_transformer::interner::Interner;
    ///
    /// let mut interner = Interner::new();
    /// let id1 = interner.entry("nome").or_insert();
    /// let id2 = interner.entry("nome").or_insert();
    ///
    /// assert_eq!(id1, id2);
    /// ```
    pub fn or_insert(self) -> TextoId {
        match self {
            Self::Occupied(id) => id,
            Self::Vacant { entry, texto } => {
                let next = u32::try_from(texto.len())
                    .ok()
                    .and_then(|n| n.checked_add(1))
                    .expect("Limite de TextoId excedido");

                let id = TextoId(NonZeroU32::new(next).unwrap());

                let key = *entry.key();

                texto.push(key);
                entry.insert(id);

                id
            }
        }
    }
}
