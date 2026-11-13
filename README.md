# Validador offline de XML SDP

Ferramenta em Rust para validar XMLs do Sistema de Desempenho Portuário da
ANTAQ. Suporta os leiautes de porto organizado (`SDPv2.xsd`) e instalação
autorizada/TUP (`SDPTup.xsd`), com diagnósticos que apontam tags, valores e
campos ausentes no documento original.

O escopo desta entrega é a **CLI de validação offline**. A ferramenta não
modifica arquivos. Transformação, serialização de saída e GUI não fazem parte
deste escopo; os módulos reservados para escrita permanecem sem implementação.

## Executar

Requer Rust 1.93 ou posterior (edição 2024 e `std::fmt::from_fn`).

```console
cargo build --release
cargo run --release -- arquivo.xml --sem-cores
cargo run --release -- --sem-cores porto.xml terminal.xml
```

Opções podem aparecer antes ou depois dos caminhos. `--help` mostra a ajuda;
`--` encerra as opções, permitindo nomes iniciados por hífen.
Sem `--sem-cores`, ANSI é ativado somente quando stderr é um terminal.

Cada arquivo é processado até o fim da validação semântica; uma falha de
leitura ou parsing não impede os arquivos seguintes. Sucessos vão para stdout;
diagnósticos, com o caminho do arquivo, vão para stderr.

| Código de saída | Resultado |
| --- | --- |
| 0 | Todos os documentos satisfazem as regras locais. |
| 1 | Há diagnósticos semânticos. |
| 2 | Houve erro de uso, I/O ou parsing. |

Em lote, prevalece o maior código. O catálogo de códigos ISO é incorporado ao
executável e carregado uma vez. Cada arquivo tem interner e mapas próprios,
liberados antes da leitura do próximo, evitando manter vários XMLs grandes
simultaneamente na memória.

## Cobertura

As regras estão documentadas nos checklists dos módulos e os enunciados
compartilhados ficam em `validation::regras_de_negocio`. A fonte principal é o
Manual SDP, 5ª edição, dezembro de 2025, complementado pelas revisões de
leiaute e decisões explícitas registradas no projeto.

A passagem verifica:

- XML bem-formado, namespace, tags permitidas, cardinalidade, valores
  numéricos, enumerações e datas;
- identificação do informante, embarcação, responsáveis e passageiros;
- cronologia e presença de datas e cargas conforme a operação;
- paralisações: códigos locais, início anterior ao término, contenção na
  estadia e duração total menor que a da operação;
- unicidade de atracação na mesma instalação, ano e berço, por IDs internados;
- CNPJ do operador, NCM, ISO incorporado, estado e natureza dos contêineres;
- relações entre sentido, operação, navegação, origem, destino e áreas;
- formatos completos e exclusivos de porto/TUP e correspondência ao informante;
- quantidade de carga geral, limites por carga e soma de unidades de
  contêiner por atracação;
- peso positivo, representação decimal, soma líquida, tara e limite de vazio;
- detalhamentos de contêineres, campos permitidos por estado e parcelas líquidas;
- cargas com características idênticas exceto os pesos, indicando o grupo
  que precisa ser consolidado, sem alterar ou somar dados do arquivo;
- coleta de resíduos: flag 0/1, certificados, itens, CNPJs, decimais exatos e
  unidades 1 (m³) ou 2 (kg).

Revisões locais relevantes: passageiros exigem ao menos uma quantidade
positiva; a cronologia é estrita após a atracação; o peso bruto de contêiner
cheio deve superar a soma líquida; CE-Mercante é opcional e, quando presente,
tem quinze caracteres. Para vazio, o CE permanece proibido e o peso bruto
total da carga é limitado a vinte toneladas. Parcelas próprias e de terceiros
são opcionais no TUP. `CodigoNCM` representa NCM/SH8.

A seção 2.2.5 do manual exige omitir navegação, origem e destino nas operações
5, 8, 10, 11 e 12. Essa regra textual foi mantida, apesar de outra seção
descrever navegação zero em alguns desses casos. A distinção está registrada
nos rustdocs. Outras decisões locais não são apresentadas como exigências do manual.

## Arquitetura e API

```text
texto UTF-8 + Interner
  → xml::reader::parse_atracacoes
  → modelos XML + mapas + SourceFile
  → validation::Validador::validar_atracacoes
  → resultado ou árvore de diagnósticos
```

O chamador mantém o texto original e fornece o mesmo `Interner` ao reader e
ao validador. Textos repetitivos são `TextoId`; os demais são empréstimos
`&str`. A ordenação de IDs é interna à tabela, não uma ordenação lexical.

```rust
use xml_transformer::{
    interner::Interner,
    validation::Validador,
    xml::reader::parse_atracacoes,
};

let xml = r#"<Movimento xmlns="SDPv2.xsd"/>"#;
let mut interner = Interner::new();
let parsed = parse_atracacoes(xml, &mut interner)?;
let validador = Validador::com_dados_incorporados()?;
if let Err(diagnosticos) = validador.validar_atracacoes(&parsed, &interner) {
    for diagnostico in diagnosticos {
        eprintln!("{diagnostico}");
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

Os mapas tipados (`AtracacaoSourceMap`, `CargaSourceMap` etc.) contêm
`ElementId` de quatro bytes. `XmlSourceMap` resolve esses IDs em tokens e spans
de bytes; `SourceFile` mantém o texto e um índice de linhas. Linha e coluna
UTF-8 são calculadas sob demanda, sem percorrer novamente o documento inteiro.
Os spans funcionam também em XML sem quebras de linha.

Cada `ElementId` pertence a um único resultado de parsing. Não misture IDs ou
mapas de documentos distintos. `TextoId` pertence ao interner que o produziu;
a biblioteca permite compartilhá-lo entre documentos cujos textos permaneçam
vivos, embora a CLI opte pelo processamento sequencial com liberação por arquivo.

Na validação, orquestradores recebem entidade e mapa; regras específicas recebem
os valores e IDs relevantes; helpers puros não conhecem XML ou diagnósticos.
Unicidade e consolidação usam tabelas hash, sem comparação de todos contra todos.
A consolidação canonicaliza a ordem dos detalhamentos antes da comparação.
Pesos e quantidades de resíduos usam inteiros escalados, sem floats.

## Diagnósticos e limites

`ValidationError` contém mensagem, `details` compartilhado, labels de tipos
distintos, `relacionados` cumulativos e `alternativas` de intenção mutuamente
exclusivas. Tags ausentes são anotadas num ponto antes do fechamento do pai.
Grupos numerosos mostram contagem e até três exemplos.

Uma declaração explícita de contêiner seleciona suas regras mesmo quando
outros campos estão errados. Códigos não reconhecidos sem indicadores e fora
de apoio oferecem alternativas de representação. Erros objetivos são vermelhos;
os diagnósticos dos ramos alternativos são laranja.

O reader interrompe no primeiro erro estrutural ou lexical. Com parsing
bem-sucedido, a validação acumula os problemas semânticos e não faz I/O externo.
Datas com `Z` ou offset são normalizadas para UTC; datas sem fuso preservam
a hora local informada. O leitor não inventa um fuso para essas datas, por
isso o documento deve usar uma convenção temporal consistente.
A entrada precisa ser UTF-8 e caber no limite de offsets `u32`.
O programa não comprova existência ou vínculo em cadastros oficiais
(portos, empresas, áreas, NCM, IMDG, GISIS etc.), nem recupera informações
que não foram declaradas no XML. Passar na validação local não garante
aceitação pelo SDP.

## Verificação

```console
cargo test
cargo doc --no-deps
cargo fmt --check
```

Os testes versionados são autocontidos: usam dados sintéticos escritos no código
e não exigem XMLs de produção. `tests/cli.rs` exercita o executável com arquivos
temporários gerados pelo próprio teste.

XMLs, XSDs, o manual SDP e os testes dependentes desses arquivos são locais e
não são versionados. Quando presentes na máquina, os testes privados ficam em
`tests/local`, `tests/common` e `tests/validation.rs`. A feature `local-tests`
habilita sua compilação; ela só deve ser usada onde esses arquivos estiverem
disponíveis.

```console
cargo test --features local-tests
cargo test --release --features local-tests parses_large_generated_dpworld_tup_document -- --ignored
```

Uma cópia limpa do repositório pode executar `cargo test`, `cargo fmt --check`
e `cargo doc --no-deps` sem esses arquivos privados.
