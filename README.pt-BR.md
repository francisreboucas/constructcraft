# ConstructCraft

[![CI](https://github.com/francisreboucas/constructcraft/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/francisreboucas/constructcraft/actions/workflows/ci.yml)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/licen%C3%A7a-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Rust 2024](https://img.shields.io/badge/rust-2024_edition-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![IFC4](https://img.shields.io/badge/IFC-exporta_IFC4_%C2%B7_importa_IFC2X3%2FIFC4-0a7bbb.svg)](https://technical.buildingsmart.org/standards/ifc/)
[![Platforms](https://img.shields.io/badge/plataformas-Windows_%C2%B7_macOS_%C2%B7_Linux-lightgrey.svg)](#baixar)
[![DCO](https://img.shields.io/badge/DCO-commits_assinados-brightgreen.svg)](CONTRIBUTING.md)
[![Discord](https://img.shields.io/badge/Discord-entrar-5865F2.svg?logo=discord&logoColor=white)](https://discord.gg/y8ySJZjpK)
[![Sponsor](https://img.shields.io/badge/Apoiar-%E2%9D%A4-ea4aaa.svg?logo=githubsponsors&logoColor=white)](https://github.com/sponsors/francisreboucas)

Software livre de **Modelagem de Informações da Construção (BIM)**, escrito em Rust e independente de qualquer fornecedor comercial.

**[Baixar](#baixar)** · [English](README.md) · [Discord](https://discord.gg/y8ySJZjpK) · [Como contribuir](CONTRIBUTING.md) · [Código de conduta](CODE_OF_CONDUCT.md) · [Segurança](SECURITY.md)

Um único modelo paramétrico é a fonte da verdade. Plantas, forros, cortes, elevações, vistas 3D, legendas, tabelas e folhas são *consultas* sobre esse modelo e se atualizam automaticamente a cada alteração.

## Baixar

Grátis e sem cadastro. Cada link baixa a versão mais recente:

| Sistema | Baixar | Instalar |
|---|---|---|
| **Windows** (x64) | [**Instalador (.msi)**](https://github.com/francisreboucas/constructcraft/releases/latest/download/constructcraft-x86_64.msi) | Execute-o. Ele instala em Arquivos de Programas com atalho no menu Iniciar, abre arquivos `.ccad` e, opcionalmente, põe o `ccad` no PATH. Versões novas atualizam a instalação existente. |
| **macOS**, Apple silicon | [**Imagem de disco (.dmg)**](https://github.com/francisreboucas/constructcraft/releases/latest/download/constructcraft-aarch64-apple-darwin.dmg) | Arraste o *ConstructCraft* para *Aplicativos*. |
| **macOS**, Intel | [**Imagem de disco (.dmg)**](https://github.com/francisreboucas/constructcraft/releases/latest/download/constructcraft-x86_64-apple-darwin.dmg) | Arraste o *ConstructCraft* para *Aplicativos*. |
| **Linux**, x86_64 | [**AppImage**](https://github.com/francisreboucas/constructcraft/releases/latest/download/constructcraft-x86_64.AppImage) · [Debian/Ubuntu (.deb)](https://github.com/francisreboucas/constructcraft/releases/latest/download/constructcraft_amd64.deb) | AppImage: dê `chmod +x` e execute. `.deb`: `sudo apt install ./constructcraft_amd64.deb`. |
| **Linux**, arm64 | [**AppImage**](https://github.com/francisreboucas/constructcraft/releases/latest/download/constructcraft-aarch64.AppImage) · [Debian/Ubuntu (.deb)](https://github.com/francisreboucas/constructcraft/releases/latest/download/constructcraft_arm64.deb) | AppImage: dê `chmod +x` e execute. `.deb`: `sudo apt install ./constructcraft_arm64.deb`. |

Qual é o seu Mac? No menu Apple, *Sobre Este Mac* mostra uma linha *Chip* com *Apple M…* nos Macs com Apple silicon e uma linha *Processador* com Intel nos Macs Intel. A [página de download](https://constructcraft.net/#download) escolhe o arquivo do seu sistema.

Os dois pacotes para Linux trazem o atalho e o ícone; o `.deb` também registra o tipo de arquivo `.ccad` e instala o `ccad` em `/usr/bin`. As versões para Linux pedem glibc 2.35 ou posterior (Ubuntu 22.04, Debian 12 e mais novos) e driver Vulkan ou OpenGL.

A página da [última versão](https://github.com/francisreboucas/constructcraft/releases/latest) tem todos os arquivos, também o `.zip` portátil para Windows, os arquivos compactados `.tar.gz` (o do macOS traz o `ccad`) e um `SHA256SUMS.txt` para conferi-los. As [versões anteriores](https://github.com/francisreboucas/constructcraft/releases) continuam lá.

**Primeira abertura.** O instalador e os executáveis do Windows são assinados digitalmente (veja a [política de assinatura de código](#política-de-assinatura-de-código)); enquanto o certificado ganha reputação, o SmartScreen ainda pode avisar: escolha *Mais informações → Executar assim mesmo*. As compilações do macOS ainda não são assinadas. No macOS, o Gatekeeper bloqueia a primeira abertura: clique com o botão direito (Control-clique) no aplicativo e escolha *Abrir* ou, no macOS 15 e posteriores, tente abrir uma vez e clique em *Abrir Mesmo Assim* em *Ajustes do Sistema → Privacidade e Segurança*.

## Familiar por design

Os menus, atalhos, painéis e ferramentas que suas mãos já conhecem. Produtivo desde o primeiro dia, sem reaprender.

O ConstructCraft segue as convenções comuns às ferramentas CAD e BIM de desktop: barra de acesso rápido no canto superior esquerdo, ribbon com abas, paleta de propriedades e navegador de projeto encaixados na lateral, e atalhos de teclado padrão e de duas letras. São convenções gerais; o ConstructCraft não copia ícones, arte ou textos de nenhum produto (veja [Independência](#independência)).

A dica de cada botão mostra o atalho, o que ele faz, por que está indisponível quando aparece acinzentado e o comando do motor por trás dele. **Ctrl+Shift+P** abre uma paleta que encontra qualquer comando, vista ou comando do motor pelo nome; **Ctrl+F1** recolhe o ribbon até as abas. Veja o [guia da interface](docs/interface.pt-BR.md).

## O nome

O projeto começou como **ConstructCAD** e agora se chama **ConstructCraft**. O "CAD" fazia algumas pessoas esperarem um programa de desenho ou de modelagem 3D genérico, uma ferramenta para traçar linhas e esculpir sólidos, e não um software BIM, em que paredes, pisos, portas e ambientes são elementos da construção com seus próprios dados, e plantas, cortes e tabelas saem de um único modelo. O novo nome diz do que o projeto trata: construir um modelo da edificação com cuidado.

*Craft* (ofício, feito com as mãos) representa esse cuidado, e a onda de programas que se dizem *craft*: ferramentas feitas com atenção aos detalhes, que respeitam quem as usa e o trabalho que fazem com elas. O ConstructCraft mantém o seu lema, **familiar por design**: o novo nome não muda nada em como ele funciona nem em onde as coisas ficam.

Só o nome mudou. A linha de comando `ccad`, os arquivos `.ccad`, `.ccomp`, `.ccmacro` e `.ccgraph`, a tabela `ccad` do Lua, o servidor MCP e os conjuntos de propriedades IFC `ConstructCAD_*` continuam com os mesmos nomes, então projetos, scripts e modelos exportados antes continuam funcionando (veja o [changelog](CHANGELOG.md)).

## Capturas de tela

![Planta e vista 3D da casa de exemplo: planta em L com uma parede em arco sob um telhado de quatro águas](docs/screenshots/pt-BR/hero.png)

A casa de exemplo, construída por [`examples/showcase.ccmacro`](examples/showcase.ccmacro) só com comandos do motor e salva em [`examples/showcase.ccad`](examples/showcase.ccad); as imagens são capturadas do aplicativo por [`scripts/screenshots.ps1`](scripts/screenshots.ps1).

| | |
|---|---|
| [![Vista 3D do telhado sobre a parede em arco](docs/screenshots/pt-BR/roof-3d.png)](docs/screenshots/pt-BR/roof-3d.png) | [![Planta hidrossanitária](docs/screenshots/pt-BR/plumbing.png)](docs/screenshots/pt-BR/plumbing.png) |
| Telhado de quatro águas calculado pelo esqueleto reto sobre a parede em arco | Planta hidrossanitária: água fria e esgoto sobre a arquitetura esmaecida, com diâmetros e declividades |
| [![Planta elétrica e diagrama do quadro](docs/screenshots/pt-BR/electrical.png)](docs/screenshots/pt-BR/electrical.png) | [![Planta de forma e vista das armaduras](docs/screenshots/pt-BR/structure.png)](docs/screenshots/pt-BR/structure.png) |
| Planta elétrica com circuitos e eletrodutos, ao lado do diagrama unifilar e do quadro de cargas | Planta de forma com pilares, vigas e sapatas identificados, e as armaduras em 3D |
| [![Planta de reforma e planta filtrada](docs/screenshots/pt-BR/graphics.png)](docs/screenshots/pt-BR/graphics.png) | [![Corte](docs/screenshots/pt-BR/section.png)](docs/screenshots/pt-BR/section.png) |
| Fases de obra (existente, demolido, novo) e filtros de vista colorindo as paredes pela espessura | Corte com hachura, níveis e o telhado |
| [![Folha](docs/screenshots/pt-BR/sheet.png)](docs/screenshots/pt-BR/sheet.png) | [![Tabela de ambientes](docs/screenshots/pt-BR/schedule.png)](docs/screenshots/pt-BR/schedule.png) |
| Folha A3 com a planta, um corte e a vista 3D | Tabela de ambientes com totais |

## Executar

```sh
cargo run --release -p cc_app                                    # projeto novo
cargo run --release -p cc_app -- examples/casa_simples.ccad      # abrir um projeto
cargo run --release -p cc_app -- examples/casa_simples.ccad --view "Elevação: Sul"
cargo run --release -p cc_app -- --lang en                       # interface em inglês
cargo run -p cc_core --example casa_simples                      # regenerar o exemplo
```

### Aviso de atualização

Ao iniciar (no máximo uma vez por dia) o aplicativo consulta a API do GitHub pela última versão e, se ela for mais nova, mostra um aviso com o link da sua página; ele nunca baixa nada. Desligue em *Arquivo → Sobre* ou com `CONSTRUCTCAD_NO_UPDATE_CHECK=1`; veja [o guia da interface](docs/interface.pt-BR.md#aviso-de-atualização) sobre o que é contatado. `ccad --version` mostra a versão.

## Idiomas

A interface está disponível em **português (Brasil)**, **inglês**, **espanhol**, **francês**, **alemão**, **italiano**, **laosiano**, **polonês**, **ucraniano**, **russo**, **chinês tradicional**, **chinês simplificado** e **japonês**, com troca em *Arquivo → Idioma*. A escolha fica salva entre sessões. Na primeira execução vale o idioma do sistema, e `--lang en|pt-BR|es|fr|de|it|lo|pl|uk-UA|zh-TW|zh-CN|ja` força um idioma.

Projetos novos recebem os nomes de níveis, materiais, tipos e vistas no idioma ativo. Projetos existentes mantêm seus nomes, pois eles fazem parte dos dados. O separador decimal acompanha o idioma (vírgula ou ponto).

As traduções ficam em `assets/i18n/*.ftl`. Um teste garante que todos os idiomas têm as mesmas chaves e os mesmos placeholders. Para adicionar um idioma, crie o `.ftl` e registre-o em `Language` (`crates/cc_app/src/i18n.rs`).

## Funcionalidades (MVP)

- **Navegador de projeto**: Plantas de piso, Plantas de forro, Vistas 3D, Elevações (N/S/L/O), Cortes, Legendas, Tabelas e Folhas. Pelo menu de contexto é possível abrir, renomear, duplicar, excluir e inserir na folha ativa.
- **Modelagem em planta**:
  - Paredes em cadeia, com junções automáticas em L (meia-esquadria) e T.
  - Portas e janelas hospedadas na parede, com abertura recortada automaticamente.
  - Pisos, forros e telhados (plano ou de uma água, com beiral) por esboço ou "Das paredes".
- **Estrutura e circulação**: pilares (retangulares ou circulares, topo preso a um nível), vigas (as pontas param nas faces dos pilares estruturais e encontram outras vigas em meia-esquadria e de topo; justificação pelo centro ou pelas faces), pilares nos cruzamentos dos eixos e vigas ao longo dos eixos ou entre pilares de uma só vez, tabelas de pilares e vigas com seção, comprimento de corte, volume e material, perfis metálicos e tubulares (I, U, L, tubos), vigas inclinadas, fundações (sapatas, baldrames, radier; `IfcFooting`), lajes unidas aos apoios, modelo analítico (vista 3D e exportação JSON), armaduras com tabela de aço (`IfcReinforcingBar`), contraventamentos e treliças, plantas de forma com etiquetas de seção, escadas retas (espelhos a partir do espelho máximo do tipo) e guarda-corpos; aba *Estrutura*; exportados como `IfcColumn`, `IfcBeam`, `IfcStair`, `IfcRailing`.
- **Telhados**: plano, uma água, duas águas e quatro águas (a menor das águas que sobem de cada beiral), com cumeeira e espigões na planta; telhados inclinados saem no IFC como B-rep de faces.
- **Ambientes**: clique dentro de paredes que delimitam ambientes para criar o ambiente; contorno, área e perímetro acompanham as paredes. Etiquetas na planta, tabela de ambientes e exportação como `IfcSpace`.
- **Eixos e anotação**: eixos com bolhas em todas as plantas, nomeados em série ao serem colocados (1, 2, 3… e A, B, C… no sentido transversal), com extremidades que se alinham às bolhas dos outros eixos e o comando *Alinhar extremidades*; cotas alinhadas, lineares, ordenadas (acumuladas a partir de uma origem), angulares, radiais e de diâmetro em qualquer vista 2D (aba *Anotar*); textos de linha simples e multilinha (com quebra automática numa largura, giráveis, editados com duplo clique) com linhas de chamada de um ou dois segmentos terminadas em seta, seta aberta, triângulo, ponto, círculo, quadrado ou traço; marcas de portas e janelas etiquetadas automaticamente.
- **Snapping**: extremidades, pontos médios, sobre a parede, ângulos de 15°, Shift para ortogonal, comprimento digitado + Enter.
- **Edição**: seleção por clique ou janela (contida ou cruzada), mover arrastando, copiar com Ctrl+arrastar, excluir. Desfazer e refazer cobrem todas as edições.
- **Componentes paramétricos**: mobiliário, louças e marcenaria feitos de caixas, cilindros e perfis de retas e arcos desenhados na planta, na frente ou na lateral, extrudados ou girados (revolução), com peças de vazio recortadas das demais; todas as coordenadas e medidas são expressões sobre parâmetros nomeados (do tipo ou da instância). Editor de componentes com verificação imediata e pré-visualização em planta, vista frontal e 3D, biblioteca inicial (mesa, cadeira, cama, sofá, bacia sanitária, lavatório, armário), ferramenta *Componente* (CM), tabelas e legendas, arquivos `.ccomp` para compartilhar e exportação/importação IFC como `IfcFurniture`, `IfcSanitaryTerminal`, `IfcLightFixture` ou `IfcBuildingElementProxy`.
- **Materiais**: navegador com identidade, gráficos (sombreamento, transparência, padrões de superfície e de corte), aparência e propriedades físicas; os padrões de corte aparecem em plantas e cortes, a transparência no 3D.
- **Renderização fotorrealista** (*Vista → Gráficos → Renderizar*, `view_render`): uma vista 3D renderizada por path tracing no processador sob um céu gerado e o sol da vista, com vidro, texturas e modo argila; as configurações ficam em cada vista e a imagem é salva em PNG, JPEG, EXR ou HDR.
- **Unidades do projeto**: milímetros, centímetros ou metros, ou unidades imperiais — pés ou polegadas decimais, pés e polegadas fracionárias (`12'-6 1/2"`) ou polegadas fracionárias, até 1/64" — (e formatos de área, volume, ângulo, inclinação e moeda, inclusive pés quadrados e cúbicos, jardas cúbicas e galões americanos) com casas decimais, símbolos e agrupamento de dígitos.
- **Ferramentas de modificação**: mover, copiar, girar, espelhar, matriz, aparar/estender, dividir, deslocar, alinhar, controle de união de paredes e fixação; recortar, copiar e colar, e corresponder propriedades de tipo.
- **Navegação e auxílios de desenho**: cubo de navegação 3D, cotas e ângulos temporários editáveis (digite o comprimento, Tab para o ângulo), linhas de alinhamento, Espaço para girar ou inverter.
- **Instalações (MEP)**: tubulações e eletrodutos (trechos nivelados, inclinados ou verticais, encadeados pelas ferramentas) formando redes com conexões derivadas (joelhos, tês, cruzetas, luvas, reduções) e ligação automática de aparelhos e pontos, dimensionamento de esgoto (UHC), água (vazão e velocidade) e circuitos (disjuntor, condutor, queda de tensão), diagramas de quadro (unifilar e quadro de cargas, nas pranchas), sistemas hidrossanitários e circuitos elétricos com códigos, cores, quadros e tensões, aparelhos sanitários com UHC e pontos elétricos (tomadas, interruptores, pontos de luz, quadros) nas alturas usuais, plantas hidrossanitária e elétrica com etiquetas de diâmetro, declividade e circuitos e a simbologia usual, tabelas de tubulações e eletrodutos e quadro de cargas dos circuitos; aba *Instalações*; exportados como `IfcPipeSegment`, `IfcCableCarrierSegment`, `IfcOutlet`, `IfcSwitchingDevice`, `IfcElectricDistributionBoard` e `IfcDistributionSystem`/`IfcDistributionCircuit`.
- **Propriedades**: parâmetros de instância e da vista em seções recolhíveis; paredes com restrição de base e de topo (altura desconectada ou até um nível), linha de localização, delimitação de ambientes e estrutural.
- **Tipos**: diálogo de propriedades do tipo com duplicar e renomear, editor de camadas com função (núcleo, acabamentos), função da parede e dados de identidade (marca de tipo, fabricante, modelo, classificação, resistência ao fogo, custo…).
- **Vista 3D** (wgpu, MSAA): orbitar, pan, zoom, ajustar ao modelo e seleção por clique.
- **Elevações e cortes**: remoção de linhas ocultas, hachura de corte e níveis anotados.
- **Tabelas** de todas as categorias do modelo e de ambientes: escolha e ordem dos campos (da instância, do tipo e calculados, como comprimento, área e volume), ordenação, filtro, totais e exportação para CSV.
- **Folhas** A0–A4: carimbo com dados do projeto e viewports arrastáveis em escala; exportação vetorial para PDF e SVG (*Arquivo → Exportar PDF/SVG…*), idêntica à tela.
- **Fases de construção** (reformas): elementos são construídos e demolidos em fases ordenadas; cada vista mostra uma fase por um filtro de fase (existente, demolido, novo e temporário pela categoria, substituído ou oculto). *Gerenciar → Fases* edita fases, filtros e seus gráficos; *Modificar → Demolir* (DE) demole elementos na fase da vista.
- **Gerenciar**: informações do projeto, níveis (cada nível novo cria suas plantas) e materiais.
- **Arquivo `.ccad`**: RON versionado e legível; o salvamento é atômico.
- **Exportação IFC4** (*Arquivo → Exportar IFC…*): pavimentos, paredes com aberturas, portas, janelas, pisos, forros, telhados, estrutura e componentes como sólidos extrudados (objetos importados como conjuntos de faces na classe original), com tipos, camadas de materiais, cores e quantitativos. Gerada pelo escritor STEP do próprio ConstructCraft, sem bibliotecas IFC de terceiros.
- **Importação do SketchUp** (*Inserir → Importar SketchUp…*, do SketchUp 3 ao formato atual): componentes e grupos chegam como elementos de componente com a geometria em malhas fixas, uma peça por material, e cópias de um componente compartilham um tipo; *Carregar componente* também aceita um `.skp` como um componente para posicionar. Lido pelo crate `cc_skp` do próprio ConstructCraft.
- **Importação IFC** (*Inserir → Importar IFC…*, IFC2X3 e IFC4) sem ignorar classes: pavimentos viram níveis e eixos viram eixos; paredes, portas, janelas, lajes, forros, telhados, ambientes, pilares e vigas viram elementos nativos com tipos (existentes ou novos), materiais e cores sempre que um elemento nativo representa a geometria exatamente. Todo outro objeto físico — escadas, guarda-corpos, perfis, chapas, fachadas-cortina, tubos e conexões de instalações, terreno, classes de outros esquemas — chega como componente genérico que mantém sua classe IFC, tipo predefinido, conjuntos de propriedades e de quantidades (os seus e os do tipo, convertidos para unidades SI) e geometria exata, e é exportado de volta na mesma classe; as aberturas que o atravessam são recortadas da sua geometria. A geometria é lida pelo motor do próprio ConstructCraft: extrusões (com vazios), revoluções, varreduras de disco e de perfil, primitivas e booleanas CSG (união, diferença, interseção), recortes por semiespaço, B-reps facetados e avançados, conjuntos de faces, itens mapeados, todos os perfis paramétricos e curvas com arcos, cônicas e B-splines.
- **Arquivos do Revit**, em compilações com o recurso opcional `rvt` (*Inserir → Importar Revit…*, *Abrir* ou `rvt_import`): um projeto `.rvt` convertido para IFC4 sem o Revit pelo leitor de código aberto [rvt-rs](https://github.com/DrunkOnJava/rvt-rs) e importado como um arquivo IFC; veja [Vindo do Revit?](#vindo-do-revit).
- **Desenhos DWG e DXF de fundo** (*Inserir → Importar CAD…*, ou solte o arquivo na janela): um desenho colocado numa planta, elevação, corte ou legenda como linhas cinzas de fundo para traçar por cima, com suas camadas, blocos, arcos e textos; snap nas extremidades, pontos médios e centros; unidades, escala, rotação, cores e visibilidade das camadas nas propriedades. O DXF é lido pelo leitor do próprio ConstructCraft; o DWG pela biblioteca de código aberto acadrust.
- **Imagens e desenhos SVG** (*Inserir → Importar imagem…*, ou solte o arquivo na janela): uma imagem PNG, JPEG, GIF, BMP ou WebP (uma planta escaneada, uma foto do terreno) ou um desenho SVG colocado numa planta, elevação, corte ou legenda sob o modelo, no tamanho dado pela resolução do arquivo, com escala, rotação e posição como um desenho CAD de fundo; sai nas pranchas e nas exportações PDF, SVG e PNG, e o projeto guarda os bytes do arquivo.

## Biblioteca de componentes

O [ConstructCraft Components](https://github.com/francisreboucas/constructcraft-components) é uma biblioteca gratuita de componentes `.ccomp` prontos: mobiliário, marcenaria, louças e metais, dispositivos elétricos e iluminação, cada um com vários tamanhos como tipos. As louças trazem as unidades Hunter de contribuição e os pesos relativos, e os dispositivos elétricos o tipo e a potência, para o dimensionamento de tubulações e circuitos. Carregue um arquivo com `component_load` (ou `ccad -p casa.ccad --save run component_load path=…`) e insira com a ferramenta *Componente* (CM). Cada arquivo é gerado e verificado pelo próprio `ccad`; veja o [catálogo](https://github.com/francisreboucas/constructcraft-components/blob/main/CATALOG.md) e como [contribuir com um componente](https://github.com/francisreboucas/constructcraft-components/blob/main/CONTRIBUTING.md). A biblioteca está publicada no [3D CraftHouse](https://3dcrafthouse.constructcraft.net), com uma prévia 3D de cada componente, e o programa a lista e baixa em *Inserir → Biblioteca de componentes → 3D CraftHouse*.

## Vindo do Revit?

O jeito mais completo de trazer um modelo do Revit é exportá-lo para IFC no próprio Revit (*File → Export → IFC*, IFC4 ou IFC2x3). Abra o arquivo no ConstructCraft com *Abrir* ou *Importar IFC…* (na tela inicial, ou em *Inserir → Importar IFC…*), ou arraste-o para a janela. Pavimentos, paredes, portas, janelas, lajes, forros, telhados, ambientes, pilares e vigas chegam como elementos nativos e editáveis, com seus tipos e materiais; todo o resto chega como objetos genéricos com sua classe IFC e geometria, e a mensagem da importação informa quantos.

Sem o Revit, uma compilação com o recurso opcional `rvt` abre o próprio arquivo `.rvt`. O recurso vem desligado por padrão e os downloads publicados não o têm; compile o aplicativo com `cargo run --release -p cc_app --features rvt` (ou o `ccad` com `cargo run --release -p cc_cli --features rvt`). Depois abra o arquivo com *Abrir*, *Importar Revit…* (na tela inicial ou em *Inserir*) ou `rvt_import`. Ele é convertido para IFC4 pelo [rvt-rs](https://github.com/DrunkOnJava/rvt-rs) (o crate `rvt`, Apache-2.0), um leitor independente e de código aberto de arquivos do Revit que não precisa do Revit, num processo separado, interrompido após 10 minutos, e importado do mesmo jeito. Arquivos salvos pelo Revit 2023 a 2025 chegam mais completos. Um elemento cuja forma o conversor não consegue ler chega como sua caixa envolvente, e a mensagem da importação informa quantos. Famílias paramétricas, vistas e folhas não são trazidas. Confira o resultado com o modelo do Revit antes de confiar nele.

## Atalhos

| Tecla | Ação | Tecla | Ação |
|---|---|---|---|
| WA | Parede | SB | Piso |
| DR | Porta | FO | Forro |
| WN | Janela | TE | Telhado |
| CT | Corte | MD | Selecionar |
| ZE | Ajustar zoom | Esc | Cancelar / sair da ferramenta |
| Ctrl+Z / Ctrl+Y | Desfazer / Refazer | Delete | Excluir |
| Ctrl+S / Ctrl+Shift+S | Salvar / Salvar como | Espaço | Girar / inverter (seleção ou ao posicionar) |
| Ctrl+N / Ctrl+O | Novo / Abrir | Ctrl+W | Fechar projeto |
| RM | Ambiente | GR | Eixo |
| DI | Cota alinhada | CL | Pilar |
| BM | Viga | ST | Escada |
| RA | Guarda-corpo | CM | Componente |
| MV / CO | Mover / Copiar | RO | Girar |
| MM / DM | Espelhar (escolher / desenhar eixo) | AR | Matriz |
| TR / TX | Aparar/estender para canto / até referência | SL | Dividir |
| OF | Deslocamento paralelo | AL | Alinhar |
| PN / UP | Fixar / Liberar | MA | Corresponder propriedades de tipo |
| Ctrl+X / Ctrl+C | Recortar / Copiar | Ctrl+V | Colar |
| TT | Texto | Ctrl+Shift+P | Paleta de comandos |
| Ctrl+F1 | Recolher / expandir o ribbon | SA | Selecionar todas as instâncias na vista |
| GP / UG | Agrupar / Desagrupar | ME | Medir entre duas referências |
| DE | Demolir (na fase da vista) | CS | Criar semelhante (a ferramenta e o tipo do elemento selecionado) |

Navegação 2D: roda do mouse dá zoom e o botão do meio faz pan. Navegação 3D: arrastar com o botão esquerdo orbita, botão direito ou do meio faz pan e a roda dá zoom.

## Automação (linha de comando e MCP)

Tudo o que o ConstructCraft faz deve poder ser acionado sem a interface. O motor (`cc_engine`) expõe as operações como comandos nomeados com parâmetros JSON e um esquema JSON cada; a ferramenta `ccad` os executa pela linha de comando, por scripts ou, para agentes de IA, como servidor [MCP](https://modelcontextprotocol.io/). O mesmo catálogo atende os três. O [guia de automação](docs/automacao.md) explica tudo em detalhe e a [referência de comandos](docs/commands.md) lista cada comando com seus parâmetros. Scripts em Lua 5.4 chamam os mesmos comandos com variáveis e laços, e plugins (uma pasta com um `plugin.toml` e código Lua) acrescentam seus próprios comandos ao catálogo, cada um um passo de desfazer; veja [Scripts Lua](docs/automacao.md#scripts-lua) e [Plugins](docs/automacao.md#plugins).

```sh
ccad commands                                   # lista os comandos
ccad schema wall_create                         # parâmetros de um comando
ccad -p casa.ccad --save run wall_create start=[0,0] end=[5,0] height=2.8
ccad -p casa.ccad run elements_list category=door
ccad -p modelo.ifc --save-as modelo.ccad run project_info   # importa um IFC
ccad -p casa.ccad script obra.jsonl             # um {"command": ..., "args": {...}} por linha
```

Comprimentos em metros, ângulos em graus, pontos `[x, y]`; níveis, tipos, vistas e materiais podem ser indicados por id ou por nome. `element_get`, `type_get` e `view_get` mostram os `params` que `element_set`, `type_set` e `view_set` alteram. Cada edição é um passo de desfazer, e uma edição que deixaria o modelo inválido é recusada.

Para usar num cliente MCP (Claude Desktop, Claude Code, IDEs), registre `ccad mcp`:

```json
{ "mcpServers": { "constructcad": { "command": "ccad", "args": ["mcp"] } } }
```

ou, no Claude Code, `claude mcp add constructcad -- ccad mcp`. No Windows, se o `ccad` não estiver no PATH do cliente, use o caminho completo, por padrão `C:\Program Files\ConstructCraft\ccad.exe` (veja [Servidor MCP](docs/automacao.md#servidor-mcp)). Cada sessão do servidor tem um projeto aberto (`project_open`, `project_new`, `ifc_import`, `project_save`).

O aplicativo roda sobre os mesmos comandos: ferramentas de desenho, painel de propriedades e diálogos alteram o modelo pelo motor. *Gerenciar → Macros* grava o que você faz como uma macro (um script de comandos, salvo como `.ccmacro`) e executa macros; `ccad script` roda os mesmos arquivos.

## Arquitetura

```
crates/
  cc_core    modelo BIM (elementos, tipos, níveis, vistas), transações undo/redo, persistência .ccad
  cc_geom    geometria derivada: paredes com junções e aberturas, lajes, telhados, cortes, picking
  cc_views   geração de desenhos 2D (planta, forro, elevação, corte, axonometria, legenda), tabelas, montagem das folhas e exportação PDF/SVG
  cc_render  renderizador 3D offscreen em wgpu
  cc_raytracing renderização fotorrealista de vistas 3D por path tracing (CPU)
  cc_ifc     exportação IFC4 e importação IFC2X3/IFC4: escritor e leitor STEP, mapeamento do modelo
  cc_engine  cada operação como comando nomeado com esquema JSON, numa sessão sem GUI
  cc_plugins scripts e plugins em Lua 5.4 (em sandbox) sobre os comandos do motor
  cc_cli     `ccad`: linha de comando, scripts e servidor MCP sobre o motor
  cc_app     aplicativo eframe/egui: ribbon, navegador, propriedades, ferramentas, folhas
assets/i18n  traduções (pt-BR, en); textos de interface nunca ficam fixos no código
```

Toda alteração passa por `Document::edit`, que registra uma `Transaction` reversível. As edições contínuas (arrastar valores) são agrupadas em um único passo de desfazer.

## Desenvolvimento

```sh
cargo nextest run --workspace   # install once: cargo install cargo-nextest --locked
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
CC_UI_SNAPSHOTS=1 cargo nextest run -p cc_app snapshots    # capturas da interface (veja docs/interface.pt-BR.md)
```

Os arquivos IFC exportados são validados na CI com o [IfcOpenShell](https://ifcopenshell.org/), uma referência independente e de código aberto usada apenas nos testes:

```sh
pip install -r scripts/requirements-ifc.txt
cargo run -p cc_ifc --example export_ifc -- examples/casa_simples.ccad casa_simples.ifc
python scripts/validate_ifc.py casa_simples.ifc
```

## Próximos passos

- Componentes em série, aninhados e hospedados (em paredes e forros).
- Importação nativa (editável) de escadas e guarda-corpos.
- Executar uma macro como um único passo de desfazer; parâmetros em macros.
- Texturas de materiais.

## Apoie o projeto

O ConstructCraft é gratuito e desenvolvido no tempo livre. Se ele é útil para você, você pode patrocinar o desenvolvimento pelo [GitHub Sponsors](https://github.com/sponsors/francisreboucas):

<a href="https://github.com/sponsors/francisreboucas"><img src="https://img.shields.io/badge/Patrocinar_no_GitHub-%E2%9D%A4-ea4aaa.svg?logo=githubsponsors&logoColor=white" alt="Patrocinar no GitHub" height="32"></a>

Um café também ajuda a manter o trabalho andando:

<a href="https://www.buymeacoffee.com/francisreboucas"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy me a coffee" height="48"></a>

Ou escaneie o QR code:

<a href="https://www.buymeacoffee.com/francisreboucas"><img src="docs/images/buymeacoffee-qr.jpeg" alt="QR code da página do Buy Me a Coffee" width="150"></a>

## Política de assinatura de código

Assinatura de código gratuita fornecida por [SignPath.io](https://about.signpath.io), certificado da [SignPath Foundation](https://signpath.org).

Os arquivos do Windows de cada [versão](https://github.com/francisreboucas/constructcraft/releases) (`constructcraft.exe`, `ccad.exe`, o `.zip` portátil e o instalador `.msi`) são compilados a partir deste repositório pelo [workflow de release](.github/workflows/release.yml) em runners hospedados pelo GitHub e assinados pelo SignPath; nada compilado em outro lugar é assinado.

- Committers e revisores: os [membros deste repositório](https://github.com/francisreboucas/constructcraft/graphs/contributors); mudanças de colaboradores externos são revisadas antes do merge.
- Aprovadores: [Francis Rebouças](https://github.com/francisreboucas) aprova cada pedido de assinatura.

**Privacidade.** Este programa não transfere nenhuma informação para outros sistemas em rede, a menos que isso seja especificamente solicitado pelo usuário ou por quem o instala ou opera. As exceções, todas desligáveis ou feitas só a pedido: a verificação de atualizações (no máximo uma vez por dia, uma requisição HTTPS à API de releases do GitHub; desligue em *Sobre* ou com a variável de ambiente `CONSTRUCTCAD_NO_UPDATE_CHECK`), e o mapa, a busca de endereço e o terreno da *Localização do terreno*, a biblioteca de componentes online e o download de plugins, que acessam seus serviços só quando o usuário os abre ou usa.

## Independência

O ConstructCraft é um projeto independente. O comportamento do programa foi projetado com base em documentação pública, convenções gerais de CAD e retorno dos usuários. Os testes de referência usam valores analíticos e ferramentas de código aberto. Compilações com o recurso opcional `rvt` leem arquivos do Revit com o rvt-rs, um projeto de código aberto separado, com sua própria política de sala limpa; a compilação padrão e os downloads publicados não o incluem. O ConstructCraft não tem afiliação com a Autodesk, nem é patrocinado ou endossado por ela. Autodesk e Revit são marcas registradas da Autodesk, Inc.

> ConstructCraft is independent. Behaviour is designed from public documentation, general CAD conventions and user feedback. Reference tests use analytic values and open-source tools. ConstructCraft is not affiliated with, sponsored by or endorsed by Autodesk. Autodesk and Revit are trademarks of Autodesk, Inc.

## Licença

Copyright © Francis Rebouças <francisreboucas@gmail.com>.

O ConstructCraft é distribuído sob a [GNU General Public License v3.0 ou posterior](LICENSE) (GPL-3.0-or-later).

O ícone do aplicativo é de [Magnific – Flaticon](https://www.flaticon.com/) (veja `assets/icons/FLATICON.txt`). A fonte [Inter](https://rsms.me/inter/) incluída é © The Inter Project Authors e distribuída sob a [SIL Open Font License 1.1](assets/fonts/OFL-Inter.txt). As fontes [IBM Plex](https://github.com/IBM/plex) Sans e Mono incluídas são © IBM Corp. e distribuídas sob a [SIL Open Font License 1.1](assets/fonts/OFL-IBMPlex.txt). A renderização usa o crate [sort-raytracing](https://crates.io/crates/sort-raytracing), distribuído sob a GPL-3.0-only. A compressão de malhas Draco usa o crate [draco-core](https://crates.io/crates/draco-core), distribuído sob a Apache-2.0.
