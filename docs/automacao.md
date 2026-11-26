# Automação: motor, linha de comando e MCP

[English](automation.md)

Tudo o que o ConstructCraft faz num modelo pode ser feito sem a interface. O **motor** (`cc_engine`) reúne as operações num catálogo de **comandos** nomeados, e três clientes usam esse mesmo catálogo:

```
                     ┌──────────────────────────────┐
  aplicativo     ──▶│                              │
  ccad (CLI)     ──▶│  cc_engine: Session +        │──▶ o modelo (.ccad)
  agentes (MCP)  ──▶│  catálogo de comandos        │
  scripts Lua    ──▶│  (+ comandos de plugins)     │
                     └──────────────────────────────┘
```

Uma parede desenhada com o mouse, um `ccad run wall_create …`, um script Lua chamando `ccad.run("wall_create", …)` e um agente de IA chamando a ferramenta `wall_create` executam o mesmo código, com os mesmos padrões, verificações e desfazer. A lista completa de comandos e parâmetros está na [referência de comandos](commands.md) (em inglês, gerada do próprio catálogo); [plugins](#plugins) acrescentam seus próprios comandos ao catálogo.

## Conceitos

**Sessão.** Um projeto aberto: o documento (com o histórico de desfazer) e a geometria e os desenhos derivados dele. A CLI abre uma por execução (ou script), o servidor MCP uma por conexão, o aplicativo uma por janela.

**Comandos.** Cada comando tem um nome em `snake_case`, um objeto JSON de parâmetros descrito por um esquema JSON e um resultado JSON. Tipos:

| Tipo | Faz | Exemplos |
|---|---|---|
| query | lê o modelo | `elements_list`, `element_get`, `schedule_table` |
| edit | altera o modelo: um passo de desfazer | `wall_create`, `elements_move`, `element_set` |
| file | trabalha com arquivos ou substitui o projeto | `project_open`, `ifc_import`, `sheet_export` |

**Unidades e coordenadas.** Comprimentos em metros, ângulos em graus, pontos `[x, y]` em planta. As cotas usam as coordenadas do desenho da sua vista (X/Y da planta nas plantas; posição horizontal e altura nas elevações e cortes). Áreas e volumes nos resultados vêm em m² e m³.

**Referências.** Níveis, tipos, vistas e materiais são indicados por **id ou nome** (`"level": 3` ou `"level": "Térreo"`; maiúsculas e minúsculas não importam; um nome ambíguo dá erro pedindo o id). Folhas também podem ser indicadas pelo número (`"A101"`). Elementos são indicados por id, que `elements_list` e os resultados dos comandos de criação informam. Sem nível, vale o nível mais baixo; sem tipo, o tipo padrão da categoria.

**Ler e alterar parâmetros.** `element_get`, `type_get` e `view_get` mostram os `params` de um objeto; `element_set`, `type_set` e `view_set` recebem os campos a alterar no mesmo formato e os mesclam:

```sh
ccad -p casa.ccad run element_get id=42
ccad -p casa.ccad --save run element_set id=42 'params={"height": 2.7}'
ccad -p casa.ccad --save run element_set id=57 'params={"shape": {"Gable": {"slope": 0.5, "ridge_deg": 0}}}'
```

Objetos se mesclam campo a campo; listas e valores simples substituem; `null` limpa um valor opcional. `material_set` e `units_set` (no formato de `material_get` e `units_get`) funcionam do mesmo jeito.

**Gráficos das vistas.** O que uma vista mostra e como faz parte da vista: `view_graphics_get` lista a visibilidade e as sobreposições de cada categoria, os filtros aplicados e as sobreposições de elementos; `view_graphics_set` mostra ou oculta uma categoria e sobrepõe suas linhas em projeção e de corte (cor, espessura em mm de papel, padrão), preenchimentos de superfície e de corte (cor, padrão), meio-tom e transparência; `element_override_set` e `element_hide_in_view` fazem o mesmo para elementos numa vista. As sobreposições são objetos como `{"cut_lines": {"color": [255, 0, 0], "weight": 0.5, "pattern": "Dash"}, "cut_fill": {"pattern": "Solid", "color": [200, 200, 200]}, "halftone": true, "transparency": 0.4}` e se mesclam como `params` (`null` redefine um atributo, `reset: true` recomeça). Valem em plantas, plantas de forro, elevações, cortes, vistas 3D, folhas e exportações PDF/SVG.

```sh
ccad -p casa.ccad --save run view_graphics_set view=Térreo category=door visible=false
ccad -p casa.ccad --save run filter_create name=Longas 'categories=["wall"]' 'rules=[{"param": "length", "op": "greater", "value": 5}]'
ccad -p casa.ccad --save run view_filter_add view=Térreo filter=Longas 'overrides={"cut_lines": {"color": [255, 0, 0]}}'
ccad -p casa.ccad run filter_elements filter=Longas
```

*Filtros* (`filter_create`, `filter_set`, `filter_delete`, `filters_list`, `filter_elements`) selecionam elementos por categoria e regras sobre parâmetros — qualquer campo de tabela (`type`, `mark`, `comments`, `level`, `material`, `system`, `structural`, `width`, `height`, `thickness`, `length`, `area`, `volume`, `diameter`…) ou `elevation` (do nível do elemento) — com `equals`, `not_equals`, `contains`, `not_contains`, `begins_with`, `not_begins_with`, `greater`, `greater_or_equal`, `less`, `less_or_equal`, `has_value` e `has_no_value`, todas as regras (padrão) ou qualquer uma (`any: true`). Uma regra sobre um campo que nenhuma das categorias do filtro tem (`width` em paredes) é recusada, pois nunca passaria. Quantidades são comparadas em unidades do SI (metros); texto, sem diferenciar maiúsculas. Uma vista aplica filtros em ordem (`view_filter_add`, `view_filter_set`, `view_filter_move`, `view_filter_remove`): o primeiro filtro que corresponde vence em cada atributo, as sobreposições do próprio elemento vencem os filtros e os filtros vencem a categoria; um elemento fica oculto se estiver oculto na vista, se a categoria estiver desligada ou se algum filtro correspondente o ocultar.

*Modelos de vista* (`template_create` a partir de uma vista, `template_set`, `template_delete`, `templates_list`) guardam escala, gráficos das categorias, filtros, altura de corte das plantas, exibição de instalações (sobre a arquitetura esmaecida) e exibição estrutural, cada um com uma marca `include`. `view_template_apply` copia-os para vistas uma vez; `view_template_assign` faz as vistas seguirem o modelo: as propriedades incluídas são copiadas agora e a cada mudança do modelo (no mesmo passo de desfazer), e editá-las na vista é recusado. Excluir um modelo libera as vistas, que mantêm suas propriedades.

*Ocultar/isolar temporário* (`view_temporary` com `action` `hide`, `isolate`, `hide_category`, `isolate_category` ou `reset`) é estado de exibição da sessão, não uma mudança do modelo: não é salvo, não entra no desfazer e não é impresso (folhas e exportações mostram a vista inteira). `view_temporary_apply` o torna permanente (oculto na vista, categorias desligadas) como um passo de desfazer.

**Fases de obra.** Uma reforma é modelada em fases (`phases_list`; o modelo tem *Existente* e *Construção nova*): cada elemento do modelo é construído numa fase (`phase_created`) e pode ser demolido na mesma ou numa posterior (`phase_demolished`), como `element_get` mostra. `phase_create` (no fim ou antes de outra, `before`), `phase_set` (renomear, ou mover com `index`; uma mudança que demoliria elementos antes de construí-los é recusada) e `phase_delete` (uma fase em uso precisa de `merge_into`) as gerenciam. Elementos novos são construídos na fase de criação da sessão (`phase_creation_set`; a da vista ativa no aplicativo, senão a última), cópias também. `elements_phase_set` define `created` e `demolished` (`null`: de pé) e `elements_demolish` demole numa fase (`phase`) ou na fase de uma vista (`view`). Portas, janelas e armaduras seguem seus hospedeiros: nunca são mais antigas que eles e são demolidas com eles, a menos que demolidas antes. Cotas, textos e eixos não têm fase.

```sh
ccad -p casa.ccad --save run elements_phase_set 'ids=[12, 13]' created=Existente
ccad -p casa.ccad --save run elements_demolish 'ids=[13]' view=Térreo
ccad -p casa.ccad --save run view_set view=Térreo 'phase_filter=Mostrar anterior + demolição'
ccad -p casa.ccad run phases_list
```

Cada planta, elevação, corte, vista 3D e tabela mostra o modelo na sua fase (`phase`) com um filtro de fases (`phase_filter`, ambos com `view_set`; modelos de vista controlam o filtro, não a fase). Em relação à fase da vista um elemento é *existente* (construído antes, de pé), *demolido* (nesta fase), *novo* (construído nesta fase) ou *temporário* (construído e demolido nela); elementos construídos depois ou demolidos antes não aparecem. Os filtros de fases (`phase_filters_list`, `phase_filter_create`, `phase_filter_set`, `phase_filter_delete`; o modelo traz *Mostrar tudo*, *Mostrar completo*, *Mostrar novo*, *Mostrar anterior + demolição*, *Mostrar anterior + novo*, *Mostrar fase anterior* e *Mostrar demolição + novo*) dizem, para cada situação, `not_displayed`, `by_category` ou `overridden`, que usa os gráficos da situação definidos no projeto (`phase_graphics_get`, `phase_graphics_set`: existente esmaecido, demolido tracejado e sem hachura de corte, temporário esmaecido e tracejado), abaixo das sobreposições de elementos e dos filtros de vista e acima da categoria. Uma vista cuja fase deixa elementos de fora tem geometria própria: portas de fases posteriores não abrem paredes existentes e paredes só se unem com paredes da fase. Um ambiente é delimitado pelas paredes de pé ao fim da fase em que é criado, então paredes demolidas nessa fase ou antes não o delimitam. Tabelas listam o que sua fase e seu filtro mostram e oferecem os campos `PhaseCreated` e `PhaseDemolished` (que também servem em regras de filtros, como `phase_created` e `phase_demolished`).

**Organização do navegador de projeto.** O navegador de projeto agrupa as vistas apenas por tipo, a menos que um esquema esteja em uso. `browser_scheme_create` cria um esquema com nome (`group_by`: os critérios, do grupo mais externo para dentro, entre `view_type`, `level`, `discipline` e `phase`; padrão `["view_type", "level"]`) e o põe em uso, salvo com `use_it` falso; `browser_scheme_set` o renomeia ou muda os critérios, `browser_scheme_delete` o remove e `browser_scheme_use` escolhe o esquema que o navegador segue (sem `scheme`: o padrão). `browser_schemes_list` lista os esquemas, e `browser_tree` devolve a árvore que o navegador mostra — linhas de grupo com `group`, `name`, `value`, `views` e `children`, linhas de vista com `view` e `name` — para o esquema em uso, outro `scheme` ou qualquer `group_by`. Uma vista sem valor para um critério (um corte não tem nível) fica ao lado dos grupos daquele critério.

**Opções de projeto.** Alternativas para uma parte do modelo ficam em conjuntos de opções (`options_list`): `option_set_create` cria um conjunto com as opções nomeadas em `options` (a primeira é a principal), e `option_create`, `option_set` (renomear, `primary: true`, mover com `index`), `option_delete` (não a principal; exclui os elementos da opção), `option_set_rename` e `option_set_delete` (com todos os elementos das suas opções) os gerenciam. Um elemento pertence ao modelo principal ou a uma opção (`design_option` em `element_get`); `elements_option_set` passa elementos para uma opção ou de volta com `null` (portas, janelas e outros hospedados seguem o hospedeiro; anotações não têm opção). Cada planta, elevação, corte, vista 3D e tabela mostra o modelo principal e uma opção por conjunto: `view_options_set` escolhe uma, ou `null` para automática (`design_options` em `view_get` mostra a escolha e o que aparece); modelos de vista podem controlá-la (`design_options` e `include.design_options` em `template_set`). Automática é a opção em edição quando ela está nesse conjunto (`option_active_set`, estado da sessão, não salvo: elementos novos do modelo entram nela), senão a principal. Elementos de opções que a vista não mostra ficam fora do seu desenho, tabela e geometria: paredes se unem e ambientes são delimitados só entre o que aparece junto. `option_accept_primary` passa os elementos da opção principal para o modelo principal e exclui o conjunto com as outras opções e seus elementos. A exportação IFC grava o modelo principal e só as opções principais, cada elemento de opção com `ConstructCAD_DesignOption` (`OptionSet`, `Option`, `Primary`), que a importação lê de volta como conjuntos e opções.

```sh
ccad -p casa.ccad --save run option_set_create name=Entrada 'options=["Varanda", "Marquise"]'
ccad -p casa.ccad --save run elements_option_set 'ids=[31]' option="Entrada: Marquise"
ccad -p casa.ccad --save run view_options_set view=Térreo set=Entrada option=Marquise
```

**Escalas.** A `scale` de uma vista é o denominador `N` de 1:`N`. `view_create`, `view_set`, `template_set` e `callout_create` também a aceitam como texto: `"1:50"` ou, em projetos imperiais, um rótulo como `"1/4\" = 1'-0\""` (48) ou `"1\" = 20'"` (240). `view_get` acrescenta `scale_text`, a escala como o projeto a escreve (`1:50`, ou `1/4" = 1'-0"` em projetos imperiais), e a escala padrão de uma chamada de detalhe num projeto imperial é uma escala imperial.

**Sistemas de medida.** Os parâmetros e resultados dos comandos são SI, quaisquer que sejam as unidades do projeto (metros, metros quadrados e cúbicos, graus, quilogramas; inclinações em porcentagem quando o comando diz), assim como os arquivos, o IFC e o DXF; `project_info` informa o `system`. O que um comando escreve como texto segue o projeto: num projeto imperial, os tipos que ele cria sob demanda recebem nomes e medidas em polegadas redondas (`Rampa 4'-0"`, `Rodapé 4"`, `Eletrocalha 8"×4"`, `Condutor Ø3/8"`, tipos de porta `2'-8" x 6'-8"`, perfis `I 8"x8"x1/4"x3/8"`; padrões como alturas de parede e de pavimento, cotas de tubos e dispositivos, quedas de esgoto, cobrimento de armaduras e curvas de nível também são imperiais, como diz a descrição de cada parâmetro), as mensagens de verificação (`stair_check`, `ramp_check`) escrevem polegadas e inclinações por 12, e as tabelas (`schedule_table`, `schedule_export_csv`) escrevem `lb` e `lb/ft` para pesos, polegadas para espessuras, diâmetros e espelhos, e cabeçalhos sem unidades métricas. As seções de condutores continuam em mm².

Com `--lang ru` ou `CCAD_LANG=ru`, as medidas exibidas em tabelas, desenhos e exportações usam símbolos de unidades russos, como `мм`, `м²` e `м³`. O idioma controla a representação textual dessas grandezas.

**Vistas de desenho.** `view_create` com `kind: "drafting"` cria uma vista 2D independente do modelo (categoria `drafting`, 1:10 a menos que `scale` diga outra coisa): ela não desenha a geometria do modelo, só os elementos colocados nela — `text_create`, `dimension_create` (e as formas em cadeia e ordenada), `cad_import` e `image_import` a aceitam como `view`. `view_get` mostra sua `scale` e `detail`, `view_set` os altera (ela não tem fase nem estilo visual) e `view_graphics_set` oculta ou sobrepõe suas categorias. Vai para folhas com `sheet_place_view` e é exportada com `view_export` (PNG, SVG, PDF, DXF) como as outras vistas; `view_delete` exclui o que foi desenhado nela.

**Chamadas de detalhe e vistas de detalhe.** `callout_create` desenha uma chamada de detalhe numa planta, planta de forro, corte ou elevação (`parent`): `rect` são dois cantos opostos nas coordenadas de desenho da vista (metros; `y` é a altura em cortes e elevações). Ela cria uma vista de detalhe dessa região (chamada `Detalhe N` a menos que `name` diga outra coisa, na escala `scale`, por padrão uma escala usual cerca de cinco vezes maior que a da vista: 1:10 sob 1:50) do mesmo tipo da vista, que continua ligada ao modelo: segue o nível, a linha de corte ou a direção da vista mãe, mostra o modelo como ela (fase, nível de detalhe, estilo visual, gráficos) e é recortada pelo retângulo, e suas próprias linhas de detalhe, regiões, componentes de detalhe, textos e cotas são desenhados inteiros. Com `drafting`, a chamada aponta para essa vista de desenho existente (sem recorte; uma chamada por vista de desenho). A vista mãe desenha a chamada como um retângulo de cantos arredondados e uma marca: o número do detalhe na folha sobre o número da folha depois de colocado, senão a última palavra do nome sobre um traço. `view_get` mostra `crop` (`[[x mín, y mín], [x máx, y máx]]`) e `callout` (`parent`, `parent_name`, `rect`, `drafting`); `view_set` `crop` move ou redimensiona a chamada de uma vista de detalhe (são um só retângulo), recorta qualquer planta, corte ou elevação, ou com `null` volta a mostrar tudo (uma chamada precisa do seu retângulo); numa vista de desenho apontada por uma chamada, move essa chamada. `callouts_list` lista as chamadas desenhadas numa vista, `callout_remove` remove uma chamada e mantém a vista, `view_delete` de uma vista de detalhe remove sua chamada, e excluir a vista mãe deixa os detalhes como vistas recortadas comuns. As plantas não desenham a vista de detalhe de um corte ou elevação como outra marca de corte ou elevação. Arquivos cujas chamadas apontam para vistas inexistentes, ou cujos recortes não são retângulos válidos, são reparados ao abrir.

```sh
ccad -p casa.ccad --save run callout_create parent="Térreo" rect=[[2,3],[5,6]] scale=20
```

**Sol.** Plantas, plantas de forro, vistas 3D, elevações e cortes podem ter um sol: `view_set` `sun` recebe os campos a mudar (`null` o remove) e `view_get` os mostra com `derived`. `mode` é `still` (`date`, `time`), `single_day` (`date`, `from`, `to`, `sunrise_to_sunset`, `interval` de 15, 30 ou 45 minutos ou uma hora), `multi_day` (`from_date`, `to_date`, `from`, `to`, `sunrise_to_sunset`, `interval` também `day`, `week` ou `month`) ou `lighting` (`azimuth`, `altitude`, `relative_to_view`), e `conventional` põe o sol do desenho de fachadas, um `lighting` relativo à vista com azimute 225 e altura atan(1/√2) ≈ 35,26, cujos raios seguem a diagonal de um cubo por cima do ombro esquerdo de quem olha, então a sombra cai para baixo e para a direita tanto quanto o que a projeta avança (`view_get` o mostra como `lighting` com `derived.conventional`); todo modo tem `frame` (o quadro do estudo mostrado), `show_path`, `shadows` e `ground` (o nível que recebe as sombras). As sombras são desenhadas nas vistas 3D e, nos estilos visuais `shaded`, `consistent_colors` e `textures`, nas plantas, elevações e cortes, então `view_export` e as pranchas as levam (a planta baixa as projeta a partir do modelo inteiro, o corte a partir do que está além da sua linha, nenhum dos dois sobre o que é interno; as plantas de forro não as desenham). Datas são `AAAA-MM-DD` e horas `HH:MM`, locais do terreno; outro modo começa dos seus valores padrão e mantém a data, e um campo que o modo não tem é recusado. `derived` dá o azimute (horário a partir do norte verdadeiro) e a altura do quadro atual, sua data e hora, o nascer e o pôr do sol, o número de quadros e se o projeto tem localização (senão, o equador em Greenwich). `sun_position` responde para uma vista ou para qualquer `date` e `time` (opcionalmente em outra `latitude`, `longitude` e `utc_offset`), `sun_frames` lista os quadros de um estudo. `view_set` `shading` muda como essas vistas sombreiam as faces e projetam sombras nos estilos `shaded`, `consistent_colors` e `textures` (2D e 3D, não em `view_render`): `ambient`, a luz das faces viradas para longe da luz (de 0,3 a 1, padrão 0,6), e `shadow`, o quanto as sombras projetadas são escuras (de 0, nenhuma, a 1, preta; padrão 0,55; o cinza de uma sombra no estilo sombreado é `1 - shadow`); `view_get` o mostra (`null` nas vistas que não aceitam sol). Predefinições com nome guardam um modo: `sun_preset_save` (de uma `view` ou de um `sun`; `replace` para sobrescrever), `sun_presets_list`, `sun_preset_rename`, `sun_preset_delete`, e uma vista toma uma com `"sun": {"preset": "Verão"}`. `site_set` `utc_offset` define o fuso horário em relação ao UTC (horas, de -12 a 14; `null` para o fuso nominal da longitude, mostrado em `utc_offset_used`).

```sh
ccad -p casa.ccad --save run view_set view="3D 1" 'sun={"mode":"single_day","date":"2026-12-21","interval":"minutes_30","shadows":true}'
```

**Linhas de detalhe.** `detail_line_create` desenha linhas 2D próprias de uma vista numa planta, planta de forro, elevação, corte, legenda ou vista de desenho (`view`), nas coordenadas de desenho da vista em metros (como notas de texto e cotas): `curves` aceita as mesmas formas dos esboços (`line`, `arc` por `bulge` ou `through`, `circle`, `ellipse` ou parte dela, `rectangle`, `polygon`), cada curva um elemento (categoria `detailline`; uma polilinha são os seus segmentos), tudo num passo de desfazer, e `style` é um dos estilos de linha internos `thin` (padrão), `medium`, `wide`, `hidden`, `dashed`, `center` e `dotted`. Só a sua vista as desenha, por cima do modelo; `view_delete` as exclui com ela, e vistas 3D e a exportação IFC as deixam de fora. `element_get` mostra `params.curve` (a curva como `Line`, `Arc`, `Circle` ou `Ellipse`) e `params.style` (`{"BuiltIn": "Wide"}`), com os derivados `length` e `style_name`; `element_set` os altera (`"style": "hidden"` também vale). Os comandos genéricos as editam: `elements_move`, `elements_copy`, `elements_rotate`, `elements_mirror`, `elements_delete`, `trim_corner` (duas linhas de detalhe, que se encontram onde suas retas, círculos ou elipses se cruzam), `trim_extend` (até outra linha de detalhe ou a linha de uma parede, viga ou eixo), `element_split` e `element_offset` (arcos e círculos concêntricos; elipses não têm deslocamento). A exportação DXF as põe numa camada com o nome do seu estilo (`THIN`, `CENTER`…).

```sh
ccad -p casa.ccad --save run detail_line_create view=Térreo style=dashed 'curves=[{"kind":"circle","center":[2,3],"radius":0.5}]'
```

**Estilos de linha.** Além dos estilos internos, um projeto tem estilos de linha próprios (nome, espessura no papel em mm, cor, tracejado), guardados como configurações do projeto: `line_styles_list` lista os dois (internos pelo código, os do projeto pelo nome, com `weight`, `color`, `pattern` e `used_by`), `line_style_create` adiciona um (`name` único e diferente dos códigos internos, `weight` de 0,05 a 5 mm, padrão 0,25, `color` `[r, g, b]`, `pattern` `solid`, `dash`, `short_dash`, `dot` ou `dash_dot`), `line_style_set` o altera (as linhas desenhadas com ele acompanham) e `line_style_delete` o exclui, passando as linhas de detalhe e contornos de regiões que o usam para `replace_with` (padrão `thin`) no mesmo passo de desfazer. Onde um comando aceita um estilo de linha (`style` de `detail_line_create`, `boundary` de uma região, `element_set`) um estilo do projeto é nomeado como um interno; `params.style` então mostra `{"Project": <id>}`. Um arquivo cujas linhas usam um estilo que não existe mais recebe o estilo fino ao ser aberto.

**Regiões preenchidas e de máscara.** `filled_region_create` desenha uma área 2D numa planta, planta de forro, elevação, corte, legenda ou vista de desenho (`view`), nas suas coordenadas de desenho em metros: as `curves` (como nos esboços) fecham laços, o maior é o contorno e os que estão dentro dele são aberturas, tudo num passo de desfazer. Ela é preenchida como o seu tipo diz e contornada no estilo de linha `boundary` (padrão `thin`; `none` para nenhum contorno); `type` escolhe um tipo de região preenchida, senão o primeiro do projeto, criado com a primeira região (linhas diagonais sobre branco). `filled_region_type_create` cria outro (padrões `foreground` e `background` — `none`, `solid`, `diagonal`, `diagonal_back`, `crosshatch`, `horizontal`, `vertical`, `grid`, `brick`, `concrete`, `sand`, `insulation`, `earth` — cada um com sua cor; um padrão `solid` preenche com a sua cor e um fundo `none` deixa o modelo visível entre as linhas do padrão); `type_set` altera o seu `fill`. `masking_region_create` desenha uma área branca opaca que esconde as linhas do modelo sob ela (sem contorno, a menos que `boundary` nomeie um estilo). As regiões são desenhadas só na sua vista, em ordem de id, por cima do modelo e sob as anotações da vista (linhas de detalhe, cotas, textos e, nas plantas, etiquetas, eixos e marcas de corte); vistas 3D e a exportação IFC as deixam de fora. `element_get` mostra `params` (`outline`, `openings`, `sketch`, `boundary`) com os derivados `area`, `perimeter` e `boundary_name`; `element_set` troca o tipo ou `"boundary": "wide"` / `"none"`; `sketch_get` e `sketch_set` leem e redesenham o contorno; os comandos de mover, copiar, girar, espelhar e excluir valem para elas, e `view_delete` as exclui com a sua vista. As exportações escrevem os preenchimentos: SVG, PDF e PNG como polígonos (uma região com aberturas como um só anel), DXF como triângulos `SOLID` e linhas do padrão numa camada com o nome do estilo do contorno (uma região sem contorno na da sua categoria, `FILLEDREGION` ou `MASKINGREGION`).

```sh
ccad -p casa.ccad --save run filled_region_create view=Térreo boundary=wide 'curves=[{"kind":"rectangle","corner":[0,0],"opposite":[4,3]}]'
```

**Componentes de detalhe.** Um componente de detalhe é um componente 2D reutilizável para detalhes construtivos (categoria `detailcomponent`, tipo de componente `Detail`): suas peças são as peças 2D dos carimbos — `stroke` (linhas e arcos por pontos, abertos ou fechados com `closed`, com espessura `weight` em milímetros de papel, `color`, tracejado `pattern` e, fechados, um preenchimento sólido `fill` ou um padrão) e `text` e `image` — mais peças `nested` que inserem outro componente de detalhe dentro dele (`component`, `x`, `y`, `rotation`, `scale`, `mirrored`), tudo em metros no seu próprio sistema (alturas de texto também) e como números ou expressões sobre os seus parâmetros. `detail_component_create` cria um como novo tipo chamado `name` (único entre tipos e componentes): a partir de `pieces`, ou de `elements` de uma vista `view` — linhas de detalhe mantêm a pena, regiões preenchidas e de máscara viram traços fechados com o seu preenchimento (aberturas como contornos), notas de texto viram textos na altura impressa (chamadas como linhas) e componentes de detalhe viram peças aninhadas — cujo ponto `origin` (padrão: o canto inferior esquerdo da sua extensão) vira a origem do componente; com `replace` (padrão true) esses elementos são excluídos e o componente é inserido onde estavam (`placed`), tudo num passo de desfazer. `detail_component_place` insere um tipo numa planta, planta de forro, elevação, corte, legenda ou vista de desenho (`view`): a origem em `position`, girado `rotation` graus, com tamanho `scale` (0,0001 a 10000) e, com `mirrored`, espelhado antes no próprio eixo X. Só essa vista o desenha, por cima do modelo e das regiões e sob as linhas de detalhe e anotações; `view_delete` o exclui com a vista, e vistas 3D e IFC o deixam de fora. `element_get` mostra `params` (`view`, `position`, `rotation_deg`, `scale`, `mirrored`, `overrides`) com os derivados `component`, `pieces` e `nested`; `element_set` os altera (menos a vista); os comandos de mover, copiar, girar, espelhar e excluir valem para ele. `component_sheet_piece_add`, `component_sheet_piece_set` e `component_sheet_piece_delete` editam as peças de um componente de detalhe (todos os inseridos acompanham), `component_param_*` os parâmetros. Uma peça aninhada nomeia um componente (usa os valores de parâmetro do primeiro tipo dele); nomear um que contém este componente (um ciclo) é recusado, e o desenho para em 8 níveis de aninhamento e num limite de peças de qualquer forma. `component_save` grava no arquivo `.ccomp` também os componentes aninhados, e `component_load` adiciona os que o projeto não tem; `component_library_list` lista componentes de detalhe com `kind` `Detail` e `category` `detail_component`. `cad_import` com `blocks: true` traz os blocos de um DWG ou DXF como componentes de detalhe em vez de explodi-los: cada bloco inserido por translação, rotação, escala uniforme ou espelhamento vira um tipo com o seu nome (os seus próprios blocos inseridos como peças aninhadas, referências que fechariam um ciclo descartadas), cada inserção desse tipo no desenho vira um componente de detalhe inserido, e o resto (outras inserções, cotas, entidades soltas) fica no desenho importado; o resultado lista os novos `components.types` e `components.elements`. A exportação DXF (`view_export`) grava os componentes de detalhe de uma vista como blocos (um por componente; um inserido com valores de parâmetro próprios tem o seu) e `INSERT`s na camada `DETAILCOMPONENT`, os aninhados como inserções dentro dos seus blocos.

```sh
ccad -p casa.ccad --save run detail_component_create name=Tijolo 'pieces=[{"kind":"stroke","closed":true,"fill":[200,80,60],"points":[{"u":0,"v":0},{"u":0.2,"v":0},{"u":0.2,"v":0.1},{"u":0,"v":0.1}]}]'
ccad -p casa.ccad --save run detail_component_place type=Tijolo view="Vista de desenho 1" position=[1,2] rotation=90
ccad -p casa.ccad --save run cad_import path=detalhes.dwg view="Vista de desenho 1" blocks=true
```

**Desenhos CAD importados.** `cad_import` lê um arquivo DWG (R12 até 2018 e posteriores, pela biblioteca acadrust) ou DXF (ASCII ou binário, lido pelo próprio ConstructCraft) para uma planta, planta de forro, elevação, corte, legenda ou vista de desenho, num passo de desfazer, e informa o que leu: `format`, `version`, `units` (e `units_assumed`), `layers`, `entities`, `blocks`, `primitives`, `skipped` (entidades não desenhadas, por tipo), `truncated` e a extensão na vista (`extents`). Blocos são inseridos (aninhados, escalados, girados, em matriz), arcos e círculos continuam arcos, elipses, splines e contornos de hachuras viram polilinhas, textos e MTEXT mantêm posição, altura e rotação; sólidos 3D, imagens, linhas infinitas e entidades parecidas são ignorados. `position` com `align` posiciona a origem do desenho (`origin`, o padrão), o centro da sua extensão (`center`) ou a origem, a menos que fique a mais de 1 km do conteúdo da vista (`auto`); `units` substitui as do arquivo (`mm`, `cm`, `dm`, `m`, `km`, `in`, `ft`, `yd`, `mi`), e arquivos sem unidades são lidos em metros quando pequenos, senão em milímetros. O elemento começa fixado e só a sua vista o desenha, sob o modelo (categoria `cadimport`; vistas 3D e exportação IFC o deixam de fora). `element_get` mostra o posicionamento em `params` (`units`, `scale`, `position`, `rotation_deg`, `mirrored`, `hidden_layers`, `color_mode` `Grey`/`Layers`/`Black`, `halftone`), mas não o desenho; `element_set` os altera; os comandos de mover, girar, espelhar, fixar e excluir valem para ele. `cad_layers` lista as camadas e `cad_layers_set` as mostra ou oculta. `cad_explode` o transforma em elementos nativos da sua vista num passo de desfazer: linhas, polilinhas (segmentos com bulge como arcos), arcos e círculos viram linhas de detalhe, um elemento por segmento (elipses, splines e contornos de hachuras, guardados como polilinhas, saem como seus segmentos); preenchimentos viram regiões preenchidas sem contorno, de um tipo de região preenchida sólido da cor do preenchimento (chamado *Sólido #RRGGBB*, reaproveitado quando um já preenche com essa cor); textos de uma linha viram notas de texto no mesmo lugar, altura, rotação e alinhamento, de um tipo de texto do seu tamanho no papel na escala da vista (reaproveitado quando um tem esse tamanho). Cada camada vira um estilo de linha do projeto com o nome e a cor da camada (cores claras escurecidas para o papel), reaproveitado quando já existe um estilo com esse nome; camadas ocultas no elemento ficam de fora (ele começa com as desligadas ou congeladas no arquivo ocultas), e o posicionamento é aplicado. Imagens continuam no elemento (sem as outras primitivas); sem elas o elemento é excluído. O modo de cor e o meio-tom não são levados. O resultado lista os elementos criados (`created`), as contagens `detail_lines`, `filled_regions` e `texts`, os novos `line_styles` e `types`, `kept` (o elemento quando as imagens ficam) e as primitivas ignoradas (`skipped`: pequenas demais ou fora dos limites); um desenho que geraria mais de 100.000 elementos, ou acrescentaria mais de 1.000 tipos, é recusado. Com `cad_import` `blocks: true` os blocos já são componentes de detalhe, e explodir os deixa como estão.

```sh
ccad -p casa.ccad --save run cad_import path=levantamento.dwg view=Térreo align=auto
ccad -p casa.ccad --save run element_set id=120 'params={"units": "Centimeters", "halftone": true}'
ccad -p casa.ccad --save run cad_layers_set id=120 'layers=["COTAS", "TEXTOS"]' visible=false
ccad -p casa.ccad --save run cad_explode id=120
```

**Validação e desfazer.** Cada edição é um passo de desfazer (`undo`, `redo`). Uma edição que deixaria o modelo inválido — uma porta numa parede inexistente, uma medida negativa, uma referência a nada — é recusada sem deixar rastro. Elementos fixados não são movidos, aparados nem excluídos.

**Erros.** Um comando que falha devolve `{"error": {"code": …, "message": …}}`, com a mensagem no idioma da sessão (`--lang`, `language_set`).

| Código | Significado |
|---|---|
| `unknown_command` | nenhum comando tem esse nome |
| `invalid_arguments` | os parâmetros não seguem o esquema ou não fazem sentido (comprimento zero, tipo desconhecido…) |
| `not_found` | o elemento, tipo, nível, vista ou material indicado não existe |
| `rejected` | a edição deixaria o modelo inválido ou não é permitida (fixado, em uso…) |
| `io` | falha ao ler ou gravar um arquivo |
| `internal` | um defeito; o projeto fica como estava |
| `script` | um script ou plugin Lua falhou: erro de sintaxe ou de execução, ou um limite atingido |
| `pending` | um comando de plugin continua em segundo plano, esperando um programa externo (só numa sessão que não bloqueia, veja [Programas externos](#programas-externos)) |
| `cancelled` | um comando de plugin que esperava um programa externo foi cancelado; o que ele fez foi desfeito |

## Linha de comando: `ccad`

O `ccad` é um programa separado, sem dependências gráficas (roda em servidores e na CI). Ele vem nos pacotes de release; a partir do código-fonte: `cargo build --release -p cc_cli` (o binário é `target/release/ccad`).

```text
ccad [opções] commands             lista os comandos
ccad [opções] schema <comando>     o esquema JSON dos parâmetros de um comando
ccad reference                     a referência de comandos em Markdown
ccad [opções] run <comando> [args] executa um comando
ccad [opções] script <arquivo|-> [level=<id|nome>]
                                   executa um script de comandos
ccad [opções] lua <arquivo|-> [args] executa um script Lua
ccad [opções] plugins              lista os plugins carregados com --plugins
ccad plugins install <zip> <pasta>  instala um arquivo de plugin numa pasta de plugins
ccad plugins remove <id> <pasta>    remove um plugin instalado
ccad [opções] mcp                  serve os comandos por MCP (stdio)

-p, --project <arquivo>   abre um projeto antes (.ccad, ou .ifc para importar)
--save                    salva o projeto no fim (no próprio arquivo)
--save-as <arquivo>       salva o projeto no fim em <arquivo>
--plugins <pasta>         carrega o plugin da <pasta>, ou cada plugin das suas
                          subpastas (pode repetir)
--lang <código>           idioma dos nomes padrão e mensagens (também CCAD_LANG):
                          en, pt-BR, es, fr, de, it, lo, pl, uk-UA, ru, zh-TW, zh-CN ou ja
```

Os argumentos de `run` podem ser um objeto JSON, `@arquivo.json`, `-` (entrada padrão) ou pares `chave=valor`, cujos valores são lidos como JSON quando possível e como texto caso contrário:

```sh
ccad -p casa.ccad --save run wall_create start=[0,0] end=[5,0] height=2.8
ccad -p casa.ccad --save run wall_create start=[5,0] end=[5,4] mid=[6,2]    # parede em arco passando por mid
ccad -p casa.ccad run elements_list category=door
ccad -p casa.ccad --save run view_set '{"view": "Corte 1", "scale": 100}'
ccad -p modelo.ifc --save-as modelo.ccad run project_info          # importa um IFC
ccad -p casa.ccad run sheet_export sheet=A101 path=a101.pdf
ccad -p casa.ccad run view_export view="Corte 1" path=corte.png dpi=300     # qualquer vista em PNG, SVG ou PDF
ccad -p casa.ccad run view_export view="Térreo" path=terreo.dxf             # ou em DXF, em tamanho real em mm
ccad -p casa.ccad run view_export view="Sul" path=sul.dxf hidden_lines=false   # uma elevação como exibida
```

O DXF de uma elevação, corte ou vista 3D traz as suas linhas visíveis: as linhas escondidas atrás de superfícies opacas são removidas, os preenchimentos das superfícies (sombreamento) ficam de fora e as linhas são unidas em polilinhas, enquanto preenchimentos de corte, regiões e textos ficam; o resultado diz `"hidden_lines": true`. `hidden_lines=false` grava as primitivas do desenho como exibidas, e `hidden_lines=true` limpa também uma planta ou outra vista. Uma vista grande demais para limpar é gravada como exibida, com `"hidden_lines": false` e um `warning`.

Os resultados saem como JSON na saída padrão. Erros vão para a saída de erro como JSON, com código de saída 1; erros de uso saem com 2. Sem `--save` ou `--save-as`, o arquivo do projeto não muda.

Os resultados se combinam com outras ferramentas; por exemplo, com `jq`:

```sh
parede=$(ccad -p casa.ccad --save run wall_create start=[0,0] end=[6,0] | jq .id)
ccad -p casa.ccad --save run door_create wall=$parede offset=1.5
```

### Scripts

Um script é uma lista de passos: um array JSON de objetos `{"command": …, "args": {…}}` ou um desses objetos por linha (linhas que começam com `//` são comentários). `ccad script` executa os passos em ordem numa mesma sessão e para no primeiro erro; cada passo é um passo de desfazer.

[`examples/small_house.ccmacro`](../examples/small_house.ccmacro) constrói uma casa pequena:

```text
// A small house built with engine commands only.
{"command": "wall_create", "args": {"start": [0, 0], "end": [8, 0], "height": 2.8}}
…
{"command": "door_create", "args": {"point": [2, -0.5]}}
{"command": "floor_create", "args": {"from_walls": true}}
{"command": "room_create", "args": {"point": [2, 3], "name": "Living room"}}
```

```sh
ccad --save-as casa.ccad script examples/small_house.ccmacro
```

### Contornos desenhados com curvas

Pisos, forros, telhados, radiers e massas recebem o contorno como `curves` em vez de pontos: linhas, arcos (por `bulge` ou por um ponto `through` por onde passam), círculos, elipses (inteiras ou parte delas), splines (`through` pelos pontos dados, `closed` fechada de volta ao primeiro, ou pelos pontos de controle `control` dos seus segmentos de Bézier cúbica, `p0, c1, c2, p1…`, como `sketch_get` as devolve), retângulos e polígonos regulares (estes dois viram linhas). Peças de componente também aceitam splines: o `bezier` de um ponto do perfil (`u1`, `v1`, `u2`, `v2`) faz do segmento até o ponto seguinte uma curva de Bézier cúbica com essas duas alças. Laços dentro do contorno são aberturas de pisos e telhados. O elemento guarda as curvas: `sketch_get` as devolve (ou linhas ao longo do contorno de um elemento desenhado por pontos) e `sketch_set` desenha o contorno de novo; mover, girar e espelhar o elemento leva as curvas junto, e editar diretamente os pontos do contorno as descarta.

```text
{"command": "mass_create", "args": {"curves": [{"kind": "circle", "center": [0, 0], "radius": 5}], "height": 9}}
{"command": "floor_create", "args": {"curves": [{"kind": "rectangle", "corner": [0, 0], "opposite": [10, 8]}, {"kind": "circle", "center": [5, 4], "radius": 1}]}}
```

### Terreno

`topography_create` cria uma superfície de terreno a partir de pontos medidos `[x, y, z]` (cotas acima do nível) dentro de um contorno `boundary` ou `curves`; sem nenhum dos dois, o contorno convexo dos pontos o limita. Os vértices do contorno recebem as cotas que os pontos lhes dão, e a terra fecha até uma base plana `base_depth` (2 m por padrão) abaixo do ponto mais baixo, então os cortes a cortam como um sólido. `element_get` mostra as cotas mínima e máxima, a base, a área, o volume e os avisos (`few-points`, `collinear`, `boundary-crossing`) em `derived.topography`. `topography_points_set` substitui os pontos ou altera, remove e adiciona pontos pelo índice; `sketch_set` redesenha o contorno e, com `points`, substitui os pontos no mesmo passo. Um tipo de terreno define o material da terra e o intervalo e as mestras das curvas de nível (`topography_type_create`, `type_set`).

```text
{"command": "topography_create", "args": {"points": [[0, 0, 0], [20, 0, 2], [20, 10, 2.5], [0, 10, 0.5], [10, 5, 1.2]]}}
```

`pad_create` conforma um platô num terreno: uma área plana na cota `elevation` (metros acima do nível do terreno) dentro de um contorno `outline` ou `curves`, com talude de corte onde o terreno fica acima dele e talude de aterro onde fica abaixo, cada um descendo até o terreno natural. As inclinações são a distância horizontal por unidade de altura (`cut_slope` 1 é 1:1, `fill_slope` 1,5 é 1:1,5); sem elas, o platô usa as do seu tipo (`pad_type_create`, `type_set`). O terreno `host`, por padrão, é aquele cujo contorno contém o centro do platô. O platô fica no nível do terreno, move-se com ele e é excluído com ele; os platôs de um terreno se aplicam na ordem de criação. `element_get` mostra o `cut` (corte), o `fill` (aterro), o `net` (aterro − corte), a `area` e os avisos (`off-terrain`, `coarse`, `failed`) de um platô em `derived.pad`, e o `derived.topography` de um terreno soma o `cut` e o `fill` dos seus platôs. As tabelas de platôs têm `Elevation`, `CutVolume` e `FillVolume`, que somam como volumes.

```text
{"command": "pad_create", "args": {"outline": [[6, 3], [14, 3], [14, 8], [6, 8]], "elevation": 1.2}}
```

### Cortinas de vidro

Uma cortina de vidro é uma parede cujo tipo tem uma grade de cortina em vez de camadas: linhas verticais ao longo da parede e horizontais a partir da base (a uma distância fixa, em um número de divisões ou com um espaçamento máximo, justificadas no início, no centro ou no fim), um painel em cada célula (vidro, cego, vazio ou um componente, como uma porta) e montantes nas linhas e em volta da borda. `curtain_type_create` cria o tipo e `wall_create` posiciona paredes dele, retas ou em arco (painéis planos ao longo do arco). Cada parede pode ajustar a sua grade: `curtain_grid_set` acrescenta, move e remove linhas e remove trechos delas (as células vizinhas viram um só painel), `curtain_panel_set` troca painéis e `curtain_get` lista linhas e painéis. Uma cortina desenhada dentro de outra parede abre essa parede onde está. Tabelas de paredes têm `PanelCount`, `GlazedArea` e `MullionLength`. A exportação IFC as grava como `IfcCurtainWall` feitas de `IfcPlate` e `IfcMember`, e a importação as traz de volta com a grade; uma cortina de outro programa volta como cortina quando os painéis formam uma grade simples, senão como objetos separados.

```text
{"command": "curtain_type_create", "args": {"name": "Vitrine", "curtain": {"vertical": {"spacing": 1.2}, "horizontal": {"layout": "FixedNumber", "count": 2}}}}
{"command": "wall_create", "args": {"start": [2, 0], "end": [6, 0], "type": "Vitrine", "height": 2.8}}
```

### Séries de tubos

Uma série de tubos é uma linha de produtos: suas bitolas (diâmetro externo, parede e diâmetro interno) e as conexões vendidas para ela com suas dimensões. `pipe_catalog` lista as 22 séries embutidas (Brasil, Europa, EUA) ou, com `series`, as linhas de uma série em milímetros. `pipe_series_import` copia para o projeto uma série embutida (`code`) ou um arquivo de série (`path` ou `text`, JSON em milímetros como `pipe_series_export` o escreve); `on_clash` (`skip`, `replace`, `rename`) resolve um código que o projeto já tem, e `sizes` (`["all"]` ou rótulos) também cria os tipos de tubo, com o nome da série e da bitola. `pipe_type_create` cria um tipo de uma linha, importando uma série embutida quando preciso. Um tipo de uma série mantém o diâmetro externo da linha (`type_set` aceita `series` e `size`; `series` nulo o deixa livre), e o dimensionamento usa o diâmetro interno da linha: `mep_sizing` dá a cada trecho de uma série a bitola (`size`) necessária, e `mep_size_apply` o mantém na sua série, criando o tipo que faltar. `pipe_series_delete` recusa uma série em uso, a menos que `detach`. Os tipos de tubo dos modelos são bitolas de séries, e os tubos de modelo de projetos antigos são ligados a elas ao abrir.

Onde tubos de uma série se encontram, a conexão é casada com as conexões vendidas para essa série: um joelho pega o ângulo de catálogo mais próximo (até 7,5°), um tê ou uma junção (ramal a 45° do trecho principal) as bitolas do trecho e do ramal, uma luva ou uma redução as suas bitolas. Um tê de redução que a série não vende vira um tê mais reduções no ramal, e uma redução que pula bitolas uma cadeia de até três reduções (`status: composite`, com as peças em `pieces`); uma conexão sem linha (um joelho de 60°, uma bitola que a série não tem) fica `missing`, com o motivo em `reason`. `mep_fittings` lista cada uma com o nome (`name`, "Joelho 90° PVC soldável 25 mm"), `series`, `size` e `size2`, e conta as que faltam. As conexões casadas são desenhadas em 3D com as dimensões do catálogo (regras sobre o diâmetro externo onde falta um valor), seus comprimentos de encaixe saem do comprimento de corte dos tubos (`cut_length` em `element_get`, e o campo `CutLength` das tabelas de tubos, ao lado de `Series`, `Size` e `InsideDiameter`), e a exportação IFC tipa cada uma por um `IfcPipeFittingType` por linha do catálogo; os tipos de segmento de tubo levam o conjunto de propriedades `ConstructCAD_PipeSeries`.

```text
{"command": "pipe_series_import", "args": {"code": "BR-PVC-SOLD", "sizes": ["40 mm", "50 mm"]}}
{"command": "pipe_type_create", "args": {"series": "BR-COBRE-E", "size": "22 mm"}}
{"command": "type_set", "args": {"type": "Tubo PVC soldável 40 mm", "params": {"size": "60 mm"}}}
```

### Perfis de parede

Uma parede reta pode ter um perfil de elevação: o contorno da sua face desenhado no seu próprio plano de elevação, `[u, z]` em metros ao longo da sua linha de localização a partir do início e acima da sua base, de modo que a base e o topo não precisam ser horizontais (um topo que acompanha um terreno inclinado, uma base em degraus, uma empena, um topo em arco). `wall_profile_set` recebe o contorno como `curves` (como nos esboços) ou `points`; ele deve ser um único laço fechado que não se cruza e que uma linha vertical cruza no máximo duas vezes (uma linha de base e uma de topo ao longo da parede, com extremidades e degraus verticais), sem furos, dentro do comprimento da parede (um perfil mais curto faz a parede ter só aquele trecho) e no máximo 100 m abaixo ou 1000 m acima da sua base. `wall_profile_slope` define uma base e um topo retos de uma extremidade à outra (`top_start`, `top_end`, `base_start`, `base_end`, alturas acima da base; por padrão a base em 0 e o topo na altura da parede), e `wall_profile_remove` retira o perfil. `wall_create` também aceita `profile` com curvas, e `sketch_get` / `sketch_set` leem e desenham o perfil de uma parede (`sketch_get` devolve o retângulo do seu comprimento e altura quando ela não tem perfil). A altura e a restrição de topo da parede continuam sendo a sua caixa nominal; o seu corpo, volume, plantas (cortada só onde o plano passa por ela), elevações, cortes, vistas 3D, área nas tabelas e IFC seguem o perfil. Portas, janelas e nichos que não ficam dentro dele ficam fora da parede. Quando o comprimento da parede muda, o perfil fica onde está ao longo da parede, é cortado nas extremidades da parede e acompanha uma extremidade que alcançava quando a parede cresce para aquele lado; um espelhamento o percorre ao contrário e uma divisão o divide. Paredes em arco, cortinas de vidro, paredes presas a um telhado e paredes com enquadramento (light steel ou madeira) não aceitam perfil. `element_get` o mostra em `params.profile`, com `area`, `min_z`, `max_z`, `start` e `end` em `derived.profile`.

```text
{"command": "wall_profile_slope", "args": {"wall": 12, "top_start": 3.0, "top_end": 1.2}}
{"command": "wall_profile_set", "args": {"wall": 13, "points": [[0, 0], [6, 0], [6, 3], [3, 4.5], [0, 3]]}}
```

### Extensões das camadas

Cada camada de um tipo de parede pode ter uma extensão vertical própria: `base_offset`, em metros a partir da base da parede em cada ponto ao longo dela (positivo começa mais acima, um isolamento acima do terreno; negativo desce abaixo dela, um núcleo que entra na fundação; dentro de ±100 m), e `height`, em metros a partir da base da própria camada (de 0,001 a 1000; null ou ausente: até o topo da parede). Uma camada nunca passa do topo da parede e, onde não lhe resta altura, não existe. Elas vão em `type_set` com a lista `layers` inteira como `type_get` a mostra (extensões padrão não aparecem); pisos e telhados não as aceitam. O corpo da parede fica em degraus ao longo da espessura, as plantas cortam cada camada na sua própria altura, as aberturas cortam todas as camadas que atravessam, e o volume da parede é a soma das suas camadas; `element_get` lista, em `derived.layers`, o `material`, a `thickness`, o `volume`, a `base` mais baixa e o `top` mais alto de cada camada acima da base da parede. Uniões, ambientes e as portas que a parede hospeda continuam vendo a parede inteira.

```text
{"command": "type_set", "args": {"type": "Isolada", "params": {"layers": [
  {"material": "EPS", "thickness": 0.05, "function": "Insulation", "base_offset": 1.0},
  {"material": "Concreto", "thickness": 0.2, "function": "Structure", "base_offset": -0.5},
  {"material": "Reboco", "thickness": 0.02, "function": "Finish", "height": 2.4}]}}}
```

### Paredes em spline

Uma parede pode seguir uma spline, uma curva suave de forma livre, em vez de uma reta ou de um arco (primeira etapa do recurso). `wall_create` desenha uma de `start` a `end` passando pelos pontos de `through` entre eles (no máximo 63: a curva passa por cada um, com as tangentes tiradas dos vizinhos), ou pelos pontos de controle (`control`) dos seus segmentos de Bézier cúbicos (`p0, c1, c2, p1, c1, c2, p2…`, 3n + 1 pontos para n segmentos, no máximo 64, o primeiro sendo `start` e o último `end`), como `element_get` os mostra em `params.spline.control`. A curva é a linha de localização da parede, então `location_line` desloca o corpo em relação a ela como nas outras paredes. Ela precisa ser aberta e ter pelo menos 5 cm, manter a direção onde os segmentos se unem (um canto pede duas paredes), deixar à face de dentro de cada curva um raio de pelo menos 5 cm (a curva não pode fechar mais do que meia parede, medida a partir da linha de localização, mais isso) e não cruzar a si mesma; uma curva que não atende é recusada com o motivo e o ponto.

`element_get` dá `derived.length` ao longo do eixo do corpo e, em `derived.spline`, o número de segmentos (`segments`), o menor raio de curvatura da linha de localização (`min_radius`), o menor raio que sobra para uma face por dentro de uma curva (`face_radius`) e os nós (`knots`) onde os segmentos se unem. `element_set` altera `params.spline.control` (`"spline": null` deixa a parede reta entre as extremidades, e uma parede reta aceita uma spline); mover só uma extremidade (`params.end`) leva junto o nó e a alça dessa extremidade. Mover, girar, espelhar, copiar e excluir funcionam como em qualquer parede; `element_split` divide a curva no ponto indicado e `element_offset` cria uma parede em spline ao lado (tão próxima quanto uma spline acompanha outra).

O corpo segue a curva em cordas a no máximo 0,5 mm dela: as plantas desenham o corte com as camadas ao longo dela em todos os níveis de detalhe, elevações, cortes e vistas 3D mostram as faces curvas, os ambientes são delimitados por ela, as cotas se prendem às suas faces e as tabelas dão comprimento, área e volume. As extremidades se unem a outras paredes como as de uma parede reta com a direção que a curva tem ali (cantos e encontros em T, exatos nas faces curvas), e outras paredes podem terminar no seu lado curvo.

Ainda não há, e os comandos recusam: portas, janelas e nichos em parede em spline, topo anexado (`wall_attach`), perfil de elevação, tipo de parede cortina, armadura, estrutura (framing), e aparar, estender e alinhar (a ela ou ela mesma); `strip_footings_under_walls` a deixa de fora. A exportação IFC grava a parede em spline como um `IfcWall` cujo corpo é uma casca fechada de facetas, com o conjunto de camadas do seu tipo e sem eixo; ao importar esse arquivo ela volta como um componente com a mesma forma.

```text
{"command": "wall_create", "args": {"start": [0, 0], "end": [9, 0.5], "through": [[3, 1.5], [6, -1]], "height": 2.8}}
{"command": "wall_create", "args": {"start": [0, 4], "end": [6, 4], "control": [[0, 4], [1, 4], [2, 5], [3, 4], [4, 3], [5, 4], [6, 4]]}}
{"command": "element_set", "args": {"id": 14, "params": {"spline": {"control": [[0, 0], [2, 2], [5, 2], [7, 0]]}}}}
```

### União de telhados

Uma ala transversal ou um avanço é unido ao telhado que encontra com `roof_join` (`roof`, a ala; `to`, o telhado principal): a ponta da ala avança sobre o telhado principal, ou é cortada, até encontrar a superfície dele, formando águas-furtadas (rincões), e o telhado principal é cortado debaixo dela. A ala encontra o telhado principal pelas arestas que estão sobre ele, ou através de um vão para o qual a sua aresta mais próxima está voltada (não mais longe do que a ala é larga ou comprida); quando as arestas ao lado dessa ponta são beirais inclinados (uma ala de duas águas com a cumeeira correndo para o telhado principal, ou uma ala de quatro águas, que ganha uma empena ali), a ponta avança, senão é só cortada. Uma ala mais baixa que o telhado principal o encontra mais acima na água; uma mais alta avança em nível até o telhado principal subir até os seus beirais. Cadeias (um avanço sobre uma ala sobre um telhado principal) e várias alas sobre um telhado funcionam. A união permanece quando qualquer dos telhados muda, e é recusada quando os telhados não se encontram, para um telhado unido a si mesmo ou quando fecharia um ciclo. Paredes presas a qualquer dos telhados acompanham os dois; áreas, telhas, estrutura, tabelas e IFC contam cada parte uma vez. `element_get` traz `derived.join` (`to`, `active`, `issue` quando a união não faz nada, os rincões em `valleys` e o seu comprimento em `valley_length`, e `cut_by`, as alas que cortam um telhado principal), e `roof_check` aponta uma união que não faz nada. `roof_unjoin` volta a construir os dois como desenhados; apagar o telhado principal libera as suas alas.

```text
{"command": "roof_join", "args": {"roof": 21, "to": 12}}
{"command": "roof_unjoin", "args": {"roof": 21}}
```

### Eletrocalhas e registros

`cable_tray_create` desenha um trecho de eletrocalha como `conduit_create` (com `circuits`, `offset`, `end_offset`); um projeto sem tipo de eletrocalha ganha um (200 × 100 mm, leito), e `cable_tray_type_create` cria outros (`width`, `height`, `style`: `ladder`, `solid_bottom` ou `perforated`). Eletrocalhas só se ligam a outras eletrocalhas, e as curvas e tês entre elas aparecem em `mep_fittings`. `valve_place` coloca um registro num tubo, num `point` perto dele ou a uma `distance` ao longo dele, de um `type` ou de uma função `kind` (`gate`, `ball`, `check`, `globe`; um tipo dessa função é criado quando não há); `valve_type_create` cria tipos de várias funções (`kinds`) de uma vez. O registro tem o diâmetro e o sistema do seu tubo, é um nó da rede e acompanha o tubo; `element_get` mostra `derived.point`, `kind` e `diameter`, e `element_set` o desloca ao longo do tubo (`params.at`).

```text
{"command": "cable_tray_create", "args": {"start": [0, 0], "end": [6, 0], "circuits": ["1"]}}
{"command": "valve_place", "args": {"pipe": 42, "distance": 0.5, "kind": "ball"}}
```

### SPDA e cabeamento estruturado

`lps_conductor_create` desenha um condutor de SPDA de `start` a `end` (sobre a superfície da cobertura sob as pontas, salvo `offset`/`end_offset`), ou uma descida da cobertura sobre `start` até 0,5 m abaixo do nível quando não há `end`; `role` (`air`, `down`, `earth`) tem padrão pelo lugar onde ele passa. `earth_ring_create` lança condutores enterrados em volta das paredes do nível (ou de `points`), `air_terminal_place` põe um captor de altura `height` na cobertura sob `position`, e `earth_electrode_place` crava uma haste de comprimento `length` abaixo do nível. Eles entram no primeiro sistema de SPDA (`system_create` com `kind: "lightning"`), criado com o primeiro deles. `lps_check` (`class` de `I` a `IV`) amostra as coberturas e informa quantos pontos a captação protege, pelos métodos da esfera rolante e da malha, com os pontos desprotegidos; `systems_list` dá os captores, as hastes e os comprimentos de condutor por função de cada SPDA.

`rack_place` insere um rack e cria o seu sistema de dados (`kind: "data"`, com o rack como `panel` e o código da marca do rack); `data_outlet_place` insere tomadas cabeadas até um `rack` (ou um `system` de dados) com `ports` conectores; `cabling_list` dá por rack as tomadas, os cabos, o comprimento estimado de cada um e os cabos acima de 90 m. `device_type_create` dá ao projeto um tipo de dispositivo embutido (`data_outlet`, `rack`, `air_terminal`, `earth_electrode` ou qualquer dispositivo elétrico). Eletrodutos e eletrocalhas levam sistemas de dados como os circuitos.

```text
{"command": "earth_ring_create", "args": {}}
{"command": "lps_conductor_create", "args": {"start": [0, 4], "end": [10, 4]}}
{"command": "air_terminal_place", "args": {"position": [5, 4], "height": 2}}
{"command": "lps_check", "args": {"class": "II"}}
{"command": "rack_place", "args": {"position": [0, 0]}}
{"command": "data_outlet_place", "args": {"position": [8, 3], "rack": 120, "ports": 2}}
```

### Perfis estruturais

Tipos de pilar e de viga são desenhados e construídos com um perfil estrutural: uma forma paramétrica padrão (`i`, `channel`, `rect_tube`, `cold_lipped_u`, `rect`…) de um material (`steel`, `cold_formed`, `timber`, `concrete`), ou um esboço. `profile_catalog` lista as séries embutidas (W, HP, IPE, HEA, HEB, perfis formados a frio da NBR 6355, madeira serrada, MLC, SHS, RHS, CHS) e, com `series` (e `search`), suas linhas; `profile_import` copia linhas para o projeto como perfis que lembram sua origem no catálogo (`on_clash`: `keep` mantém o perfil existente, o padrão, ou `rename`). `profile_type_create` cria um tipo de pilar ou de viga de um perfil, com um material da sua classe; `type_set` muda o `profile` de um tipo (um nome, ou `null` para a seção própria), `profile_rotation_deg` e `profile_mirror`. `profile_create`, `profile_set`, `profile_duplicate` e `profile_delete` (`replace_with`) editam perfis, e os tipos que os usam acompanham; `profile_get` dá o contorno e as propriedades da seção, `element_get` a `derived.section` de um pilar ou viga (área, massa por metro, momentos de inércia e módulos resistentes) e `analytical_model` a seção de cada barra. As barras de uma viga em arco seguem o arco: cada uma tem um `arc` (`bulge`, `center`, `radius`, `sweep` em graus) e o comprimento do arco em `length` (formato 3); `arc_segments` (em `analytical_model` e `analytical_export`) escreve cada uma como esse número de barras retas entre nós sobre o arco, para programas que só aceitam barras retas.

```text
{"command": "profile_import", "args": {"series": "IPE", "designations": ["IPE 200"]}}
{"command": "profile_type_create", "args": {"profile": "IPE 200", "category": "beam"}}
{"command": "beam_create", "args": {"type": "IPE 200", "start": [0, 0], "end": [6, 0]}}
{"command": "profile_set", "args": {"profile": "IPE 200", "name": "IPE 200 (cobertura)"}}
{"command": "profile_get", "args": {"profile": "IPE 200 (cobertura)"}}
{"command": "profiles_list", "args": {"material": "steel"}}
```

### Estrutura por material e pórticos por grade

`structural_type_for` (`category` pilar ou viga, `material` concrete, steel ou timber) devolve o primeiro tipo desse material, ou cria um padrão do catálogo. `grid_frame_create` coloca pilares em cada pavimento de uma faixa de níveis e vigas nos níveis superiores sobre as grades (`grids` padrão todas, `base_level`/`top_level` padrão os dois primeiros níveis, `column_type`/`beam_type` ou um `material`), com vigas secundárias opcionais em cada vão, num só passo de desfazer (no máximo 2000 elementos); devolve os ids e o número de pulados.

```text
{"command": "grid_frame_create", "args": {"base_level": "Térreo", "top_level": "Cobertura", "material": "steel", "secondary": {"parallel_to": 12, "spacing": 2.5}}}
```

`steel_building_create` cria um galpão metálico paramétrico: pórticos de pilares e vigas de cobertura em `bays` módulos, com terças e longarinas (`null` desliga uma delas), como pilares e vigas comuns num só passo de desfazer; os tipos padrão são o `W310x38,7` do catálogo. Os elementos são regenerados, mantendo seus ids, quando `element_set` muda os parâmetros do galpão; um elemento editado ou excluído diretamente fica desvinculado e não é mexido. Os elementos acompanham as fases e a opção de projeto do galpão, e uma cópia ou colagem faz os seus próprios. `element_get` lista `derived.members`, `detached` e `at_limit` (o arranjo chegou ao limite de elementos e deixou de fora as últimas terças ou longarinas).

```text
{"command": "steel_building_create", "args": {"span": 24, "eave_height": 7, "bays": 8, "roof": "mono", "girts": null}}
{"command": "element_set", "args": {"id": 42, "params": {"span": 30}}}
```

`truss_create` cria uma treliça paramétrica: um elemento hospedeiro cujos banzos, diagonais e montantes são vigas e pilares comuns, feitos num só passo de desfazer e devolvidos como `{id, members}`. `shape` é `flat`, `mono` ou `gable`; `pattern` é `pratt`, `howe`, `warren` ou, nas de duas águas, `kingpost`, `queenpost` e `fink`; `copies` treliças ficam a `spacing` de distância à esquerda da linha do vão; `material` (`steel` ou `timber`) escolhe as linhas do catálogo dos tipos padrão (concreto é recusado). Os elementos são feitos como na obra: cada banzo é uma viga contínua por trecho reto (dois banzos superiores na de duas águas); os montantes ficam entre os banzos, nas faces de banzos maciços ou dentro de perfis U abertos para a alma (o padrão do aço: perfis deitados, o banzo inferior aberto para cima e o superior para baixo); e onde a treliça termina num montante os banzos passam por cima dele e o montante da ponta final é virado, fechando as duas pontas. Como no galpão metálico, os elementos são regenerados, mantendo seus ids, quando `element_set` muda os parâmetros, e um elemento editado ou excluído diretamente fica desvinculado.

```text
{"command": "truss_create", "args": {"start": [0, 0], "end": [12, 0], "height": 2, "shape": "gable", "pattern": "fink", "panels": 6, "copies": 5, "spacing": 1.2, "material": "timber"}}
{"command": "element_set", "args": {"id": 42, "params": {"panels": 8}}}
```

`connections_create` coloca ligações de aço e madeira: `kind` é `base_plate` (em pilares), `end_plate`, `fin_plate`, `timber_bolts` ou `hanger` (em vigas); `members` os lista, senão `level` (por padrão o nível ativo) pega todos os seus pilares ou vigas. Só as extremidades que servem ao tipo ganham uma, as que já têm uma são puladas, e o resultado é `{created, skipped}` num só passo de desfazer. `params` muda os tamanhos padrão do tipo (metros; `thickness`, `width` e `length`, com 0 valendo a partir da seção, `bolt_diameter`, `rows`, `columns`, `spacing`, `edge`, `anchor_length`). Depois, `element_set` edita os tamanhos, não o elemento, a extremidade nem o tipo. As tabelas têm os campos `ConnectionKind`, `PlateMass` (kg), `BoltCount` e `BoltDiameter`.

```text
{"command": "connections_create", "args": {"kind": "end_plate", "members": [12, 13], "params": {"rows": 3, "bolt_diameter": 0.02}}}
{"command": "element_set", "args": {"id": 42, "params": {"params": {"thickness": 0.02}}}}
```

`wall_framing_create` estrutura paredes retas em light steel frame ou madeira: um elemento de ossatura por parede, dono de pilares comuns (montantes, montantes duplos, montantes curtos) e vigas (guias ou soleiras, vergas, contravergas, bloqueadores), feitos num só passo de desfazer e devolvidos como `{framings: [{id, wall, members}]}`. `system` é `light_steel` (padrão) ou `wood`; `spacing` é o espaçamento dos montantes em metros (0,2 a 1,2; 0,4 e 0,6 por padrão); `stud_type`, `track_type` e `header_type` aceitam id ou nome (padrão: os tipos do catálogo do sistema); `double_top` (madeira) e `blocking` são booleanos. Paredes curvas e já estruturadas são recusadas antes de qualquer alteração. Os elementos são regenerados, mantendo seus ids, quando `element_set` muda os parâmetros da ossatura ou quando a parede, suas portas ou janelas mudam; um elemento editado ou excluído diretamente fica desvinculado. As guias e soleiras padrão são as linhas do catálogo giradas 90 graus, deitadas (a largura da seção é a sua altura); um tipo dado é usado como desenha. A ossatura e seus elementos acompanham as fases, a opção de projeto e o nível da parede. Montantes não são unidos onde uma abertura chega à ponta da parede, onde duas aberturas ficam a menos de uns quatro montantes uma da outra, nem no canto entre duas paredes estruturadas; dividir uma parede estruturada reestrutura só a parte que fica com a ossatura.

```text
{"command": "wall_framing_create", "args": {"walls": [12, 13], "system": "wood", "blocking": true}}
{"command": "element_set", "args": {"id": 42, "params": {"spacing": 0.4}}}
```

`beam_create` desenha uma viga em arco com `mid` (um ponto do arco), `bulge` ou `radius` (positivo no sentido anti-horário; `major_arc` escolhe o arco maior), como `wall_create` desenha paredes em arco; `element_get` devolve então `center`, `radius`, `sweep` e o `true_length` ao longo do arco. `rebar_create` arma uma viga ou pilar: `top`, `bottom` (vigas) ou `perimeter` (pilares) com um `count`, `sides` (barras de pele nas duas faces laterais de uma viga: um `count` por face, ou um `spacing` ao longo da altura) e `stirrups` a cada `spacing`, cada estribo fechado por dois ganchos em um canto (`hook`: 135 graus por padrão, de 90 a 180, 0 para nenhum; `tail`: a ponta reta, 10 diâmetros da barra por padrão, no mínimo 5 cm) que entram no seu comprimento e aparecem na tabela de dobramento (`stirrup 14x44 hook 135° 5 hook 135° 5`). Em uma viga em arco as barras seguem o arco, cada uma com o comprimento do arco no seu raio, e os estribos são radiais. As barras que não cabem lado a lado vão para uma segunda camada, e as barras ao longo de uma viga mais compridas que `max_length` (padrão 12 m; 0 para barras inteiras) são emendadas com traspasse `lap` (padrão 40 diâmetros), com as emendas vizinhas defasadas.

```text
{"command": "beam_create", "args": {"start": [2.5, 0], "end": [0, 2.5], "radius": 2.5}}
{"command": "rebar_create", "args": {"host": 42, "layout": "bottom", "count": 6}}
{"command": "rebar_create", "args": {"host": 42, "layout": "sides", "spacing": 0.15}}
{"command": "rebar_create", "args": {"host": 42, "layout": "stirrups", "spacing": 0.12}}
```

Com `anchor` as barras superiores, inferiores e de pele de uma viga continuam para dentro dos pilares e blocos de coroamento em que as suas pontas se apoiam, ao longo da sua própria linha ou círculo: até a face oposta do apoio menos o cobrimento, no máximo o comprimento de ancoragem depois da face de entrada (`true` para 40 diâmetros da barra, ou um número de diâmetros de 10 a 100). Onde o apoio é raso demais para isso, as barras superiores e inferiores terminam em um gancho de 90° cuja perna completa o restante, com no mínimo 8 diâmetros e dentro da altura do apoio menos o cobrimento: `anchor_hook` `auto` (barras superiores para baixo, inferiores para cima), `down`, `up` ou `none`; as barras de pele ficam retas. Onde outra viga continua alinhada do outro lado do apoio (uma viga contínua, ou os arcos de uma viga em anel), as barras das duas passam meio traspasse além do seu meio, de modo que se traspassam por `lap` dentro dele, ou chegam à face oposta menos o cobrimento quando ele é mais raso. A ancoragem entra no comprimento das barras (e portanto nas posições e tabelas) antes de elas serem emendadas; `lap_at_joins` continua valendo onde uma viga encontra outra sem pilar. `rebar_typical` recebe `anchor` para as barras superiores e inferiores das vigas, e `element_set` altera a `anchorage` de um conjunto (`{"length": 30, "hook": "none"}`, ou `null`):

```text
{"command": "rebar_create", "args": {"host": 42, "layout": "bottom", "count": 3, "anchor": true}}
{"command": "rebar_create", "args": {"host": 42, "layout": "top", "anchor": 50, "anchor_hook": "down"}}
```

Um pilar que continua uma parede cilíndrica tem um perfil `annular_sector`; `rebar_create` recebe então as barras face a face (as faces são listadas, em ordem, no `derived.faces` do pilar), estribos que seguem o contorno, grampos e lances:

```text
{"command": "profile_create", "args": {"name": "P1", "material": "concrete", "shape": "annular_sector", "params": {"r_in": 2.0, "r_out": 2.2, "sweep": 30}}}
{"command": "profile_type_create", "args": {"profile": "P1", "category": "column"}}
{"command": "column_create", "args": {"type": "P1", "position": [0, 2.1]}}
{"command": "rebar_create", "args": {"host": 42, "layout": "perimeter", "faces": [7, 3, 7, 3], "same_type": true}}
{"command": "rebar_create", "args": {"host": 42, "layout": "stirrups", "spacing": 0.15}}
{"command": "rebar_create", "args": {"host": 42, "layout": "ties", "spacing": 0.30}}
```

Os grampos vão de cada barra entre os cantos até a barra oposta. `tie_faces` os limita às barras de algumas faces, pelo nome ou índice em `derived.faces` (no setor anular, `outer_arc`, `end_side`, `inner_arc` e `start_side`): num setor anular, `["outer_arc", "inner_arc"]` liga só as barras dos arcos externo e interno entre si, deixando de fora o grampo longo entre as barras do meio das duas faces radiais, que o estribo já segura:

```text
{"command": "rebar_create", "args": {"host": 42, "layout": "ties", "spacing": 0.30, "tie_faces": ["outer_arc", "inner_arc"]}}
```

As barras verticais de um pilar cortadas em lances (`max_length`, ou `lifts` em alturas dadas) são todas emendadas nas mesmas alturas; `stagger` emenda uma barra sim, outra não, em volta da seção, mais acima: `half_lap` (meio traspasse) ou um deslocamento em metros, no máximo `(max_length - lap) / 2`. Cada barra continua indo da base ao topo, então o primeiro pedaço de uma barra defasada é mais comprido pelo deslocamento e o último mais curto (uma emenda que fique a menos de um traspasse do topo é retirada); com `max_length` os lances são encurtados quando o primeiro pedaço mais comprido não caberia na barra comercial. Os pedaços defasados têm posições próprias. `element_set` a altera (`{"splice": {"stagger": "half_lap"}}`, `{"offset": 0.3}` ou `"none"`):

```text
{"command": "rebar_create", "args": {"host": 42, "layout": "perimeter", "count": 8, "max_length": 4, "lap": 0.5, "stagger": "half_lap"}}
```

`wall_rebar_create` arma paredes, retas ou em arco: barras horizontais (de anel) com um espaçamento ou `zones` por altura, barras verticais com pontas dobradas e conjuntos adicionais, e grampos, junto às duas faces com um cobrimento para cada. Uma parede cilíndrica recebe as suas quatro famílias de barras em um comando:

```text
{"command": "wall_rebar_create", "args": {"walls": [12, 13], "cover_exterior": 0.03, "cover_interior": 0.04, "horizontal": {"type": "Barra Ø 10,0 mm", "zones": [{"up_to": 2, "spacing": 0.10}, {"up_to": 4, "spacing": 0.12}, {"up_to": 6, "spacing": 0.15}]}, "vertical": {"spacing": 0.20, "bottom": {"across": 0.12, "along": 0.60}}, "max_length": 5}}
{"command": "wall_rebar_create", "args": {"walls": [12, 13], "vertical": {"spacing": 0.20, "offset": 0.10, "to": 1.5}}}
{"command": "wall_rebar_create", "args": {"walls": [12, 13], "ties": {"every_horizontal": 1, "every_vertical": 2}}}
```

`slab_rebar_create` arma pisos e radiers de qualquer contorno: uma malha, ou barras de anel e radiais, recortadas pelo contorno verdadeiro e em volta das aberturas, com reforço de aberturas. Uma laje circular com tampa de inspeção, armada em cima e embaixo:

```text
{"command": "floor_create", "args": {"structural": true, "curves": [{"kind": "circle", "center": [0, 0], "radius": 2.0}, {"kind": "rectangle", "corner": [-0.3, -0.3], "opposite": [0.3, 0.3]}]}}
{"command": "slab_rebar_create", "args": {"slabs": [42], "type": "Barra Ø 8,0 mm", "spacing": 0.15, "cover": 0.03}}
{"command": "slab_rebar_create", "args": {"slabs": [42], "layout": "radial", "layers": "bottom", "spacing": 0.20, "radial_count": 24, "intermediate": 1.0}}
{"command": "element_get", "args": {"id": 43}}
```

Uma sapata em anel sob uma parede cilíndrica: um tipo de sapata corrida em T invertido, uma sapata em arco, a sua armadura, e um radier circular que guarda a curva verdadeira:

```text
{"command": "type_set", "args": {"type": "Viga baldrame 20 x 40 cm", "params": {"width": 1.6, "thickness": 0.4, "haunch": {"edge_thickness": 0.2, "beam_width": 0.4, "beam_height": 0.5}}}}
{"command": "strip_footing_create", "args": {"start": [3, 0], "end": [-3, 0], "bulge": 1}}
{"command": "strip_rebar_create", "args": {"strips": [42], "transverse_spacing": 0.20, "longitudinal_spacing": 0.20, "beam": {"top": 3, "sides": 1, "stirrups": 0.15}}}
{"command": "foundation_slab_create", "args": {"curves": [{"kind": "circle", "center": [0, 0], "radius": 2.5}]}}
```

`pile_cap_rebar_create` arma blocos sobre estacas (os `caps` indicados, ou os blocos de um `level` ou do projeto ainda sem armadura), um conjunto por família num só passo de desfazer, dispostos a partir da disposição de cada bloco, de modo que acompanham o seu tipo: barras inferiores `bottom` sobre as estacas (uma faixa de `count` barras entre cada duas estacas de um bloco de três ou quatro estacas, ao longo dos lados do triângulo ou do quadrado e, com `diagonals`, nas diagonais do quadrado; `count` barras ao longo da linha de um bloco de duas estacas ou em linha; uma malha a cada `spacing` num bloco de uma estaca, circular ou anelar), apoiadas nas cabeças das estacas, com as pontas dobradas para cima nas faces laterais por `bend`; uma malha superior `top`; barras de pele `sides` em volta do contorno; e estribos `stirrups` num bloco de duas estacas ou em linha. Cada família recebe os seus valores ou `false`; blocos pequenos demais para o cobrimento voltam em `skipped`. `pile_rebar_create` arma estacas (`piles`, as estacas de `caps`, ou as de um nível ou do projeto ainda sem armadura): `count` barras (ou `spacing`) em volta da seção, de `length` abaixo da cabeça (0: a estaca inteira) até `projection` acima dela, no máximo até o topo do bloco menos o cobrimento, e uma espiral de passo `pitch` (com o comprimento exato da hélice na tabela de dobramento) ou estribos (`transverse`). Um bloco de três estacas e as suas estacas:

```text
{"command": "pile_cap_type_create", "args": {"layout": "triangle", "spacing": 1.2, "edge_distance": 0.4, "thickness": 0.8}}
{"command": "pile_cap_create", "args": {"position": [0, 0], "type": 42}}
{"command": "pile_cap_rebar_create", "args": {"caps": [43], "type": "Barra Ø 12,5 mm", "cover": 0.05, "bottom": {"count": 4, "bend": 0.3}, "sides": {"spacing": 0.2}}}
{"command": "pile_rebar_create", "args": {"caps": [43], "count": 6, "length": 6.0, "transverse": "spiral", "pitch": 0.1, "cover": 0.05}}
{"command": "bending_schedule_create", "args": {}}
```

`bracing_create` contraventa um vão de pórtico: um elemento hospedeiro cujas barras são vigas comuns, criado em um passo de desfazer e devolvido como `{id, members}`. O vão vai de `start` a `end` (os eixos dos dois pilares) ou fica entre os dois `columns`, de quem toma a base e a altura; `pattern` é `x`, `diagonal` (com `flip` e `alternate`), `v`, `chevron` ou `k`; `stories` pavimentos de altura `height` ficam um sobre o outro a partir de `base_offset`; `material` (`steel` ou `timber`) escolhe a linha de catálogo do tipo padrão das barras. `element_set` muda seus parâmetros e regenera as barras mantendo seus ids.

```text
{"command": "bracing_create", "args": {"start": [0, 0], "end": [6, 0], "pattern": "chevron", "height": 3, "stories": 4}}
{"command": "bracing_create", "args": {"columns": [12, 13], "pattern": "k", "stories": 2}}
{"command": "element_set", "args": {"id": 42, "params": {"pattern": "X"}}}
```

### Folhas e carimbos

`sheet_set` altera uma folha: `number`, `size` (`"A0"`…`"A4"`, `"ANSI A"`…`"ANSI E"`, `"ARCH A"`…`"ARCH E1"`, `"ARCH E"`, ou `{"width": …, "height": …}` em milímetros), `portrait`, `title_block` (um tipo de carimbo, ou `null` para nenhum), `fields` (os valores da folha para os campos por folha do carimbo; `null` limpa um) e `issue_date` (`AAAA-MM-DD`, ou `""`). `sheet_fields` mostra o que o carimbo da folha vai ler: cada campo com seu valor, os campos por folha com o valor desta folha e o padrão, e as peças que não se resolvem. `project_info_set` aceita `fields` (campos do projeto por nome, lidos como `{project:Nome}`).

Um carimbo é um componente cujas peças 2D desenham a folha inteira, em milímetros a partir do canto inferior esquerdo do papel. `title_block_create` copia o padrão (ou começa vazio com `empty`); `component_sheet_piece_add`, `component_sheet_piece_set` (um merge patch sobre a peça como `component_get` a mostra em `def.sheet`) e `component_sheet_piece_delete` editam seus traços, textos e imagens; várias peças são adicionadas (`pieces`), excluídas (`indices`) ou alteradas e adicionadas (`add`) em um só passo. Uma imagem não é alterada por patch: `component_sheet_piece_set` a substitui com `image_path` (um arquivo) ou `image` (base64), e `component_get` e `type_get` a mostram apenas como resumo (`format`, `width`, `height`, `bytes`), nunca em base64. `title_block_create` recusa um nome já usado por um tipo ou por um componente. A categoria é `titleblock` (`types_list`, `categories_list`). `type_delete` pode excluir o último tipo de carimbo: suas folhas ficam então sem carimbo, e uma folha sem carimbo é desenhada como papel em branco. As expressões leem os parâmetros e `SheetWidth`/`SheetHeight`, então um carimbo serve para qualquer papel; os textos têm `{campos}` (`sheet_number`, `sheet_name`, `sheet_count`, `sheet_index`, `sheet_scale`, `paper`, `project_name`, `project_number`, `client`, `address`, `author`, `date`, `issue_date`, `project:<Nome>`, `param:<Nome>`). Um campo por folha é um parâmetro de texto com `instance`.

```text
{"command": "title_block_create", "args": {"name": "Escritório"}}
{"command": "component_param_add", "args": {"type": "Escritório", "name": "Desenho", "kind": "text", "instance": true, "text": "Equipe"}}
{"command": "component_sheet_piece_add", "args": {"type": "Escritório", "piece": {"kind": "text", "x": "SheetWidth - 60", "y": 8, "text": "{param:Desenho} · {sheet_index}/{sheet_count}", "size": 2.5}}}
{"command": "component_sheet_piece_add", "args": {"type": "Escritório", "piece": {"kind": "image", "x": 15, "y": 15, "width": 30, "height": 12, "image_path": "logo.png"}}}
{"command": "view_create", "args": {"kind": "sheet", "name": "Plantas gerais"}}
{"command": "sheet_set", "args": {"sheet": "Plantas gerais", "title_block": "Escritório", "size": "A1", "issue_date": "2026-11-06", "fields": {"Desenho": "Ana"}}}
{"command": "sheet_fields", "args": {"sheet": "Plantas gerais"}}
```

**Referências entre passos.** Scripts não têm variáveis, mas um argumento pode tomar um valor do resultado de um passo anterior: `{"$step": n}` é o `id` que o passo `n` (contado a partir de 1) devolveu, e `{"$step": n, "path": "/created/0"}` qualquer valor do seu resultado, indicado por um [ponteiro JSON](https://www.rfc-editor.org/rfc/rfc6901). `{"$level": "active"}` é o nível ativo: no app, o nível da planta ativa; no `ccad script`, o nível dado como `level=<id|nome>`; sem nenhum dos dois, o nível mais baixo. Uma referência a um passo que ainda não rodou, ou a um valor que o seu resultado não tem, para o script com um erro `invalid_arguments`.

```text
{"command": "wall_create", "args": {"level": {"$level": "active"}, "start": [0, 0], "end": [6, 0]}}
{"command": "window_create", "args": {"wall": {"$step": 1}, "offset": 2, "sill": 1}}
```

```sh
ccad -p casa.ccad --save script paredes.ccmacro level="Pavimento 1"
```

Os comandos também ajudam aceitando posições onde isso importa (`door_create` e `window_create` encontram a parede mais próxima de `point`; pisos, forros e telhados acompanham as paredes com `from_walls`). Para lógica entre passos, escreva um [script Lua](#scripts-lua), chame `ccad run` a partir de um script de shell ou use um agente via MCP.

### Posições das barras e tabelas de armação

Cada barra de armadura tem uma posição que identifica sua forma (mesmo diâmetro, aço, trechos e dobras, dentro da tolerância de comprimento, 5 mm por padrão, e de 1 grau): `bar_marks_list` lista as posições com a forma (`code`, por exemplo `L 30-470`), quantidade, comprimentos, pesos e conjuntos. As formas são numeradas por partição, depois por diâmetro e comprimento do maior para o menor; `element_set` dá a um conjunto sua própria posição (`bar_mark`); `bar_marks_update` fixa as posições mostradas, para que fiquem quando o modelo muda, e `bar_marks_renumber` renumera do início (`prefix`, `start`, `keep_overrides`). `bar_marking_set` muda a tolerância e o comprimento da barra comercial (`stock_length`), que `bar_length_check` usa para listar as barras que precisam de emenda. `bending_schedule_create` cria uma tabela com uma linha por posição (o desenho da forma nas folhas, seu código no CSV) e um resumo por diâmetro (`partitions`, `group_by_partition`, `waste_percent`); `bar_tag_create` escreve quantidade, posição, diâmetro, espaçamento e comprimento de um conjunto numa planta, elevação ou corte.

```text
{"command": "bar_marks_update", "args": {}}
{"command": "bending_schedule_create", "args": {"partitions": ["PAREDES"], "waste_percent": 10}}
{"command": "bar_tag_create", "args": {"view": 45, "set": 120, "position": [1, 1.5], "multiplier": 2}}
{"command": "bar_length_check", "args": {"max_length": 12}}
```

Uma planta, elevação ou corte desenha a armadura sobre o modelo com `view_set` `rebar`: as barras que o plano de corte atravessa como pontos da largura da barra, e o resto das barras dentro da sua faixa como linhas finas (a planta até o nível abaixo, o corte até a sua profundidade e dentro da sua linha). As tags de armadura desenham o conjunto que etiquetam dentro da mesma faixa:

```text
{"command": "view_set", "args": {"view": 46, "rebar": true}}
```

### Etiquetas

As plantas etiquetam sozinhas portas, janelas, ambientes, pilares, vigas, sapatas e blocos sobre estacas. Um tipo de etiqueta (`tag_type_create`, `target` e um `style` parcial) diz o que a etiqueta escreve e como ela é: até 8 linhas (`lines`) de texto com campos `{Campo}` (um campo de tabela pelo nome, como `{Width}`, `{Height}` ou `{Comments}`, ou `{Mark}`, `{TypeName}`, `{Name}`, `{Number}`, `{Area}`, `{Section}`, `{Level}`) e um tamanho em milímetros de papel, um símbolo (`symbol`: `Circle`, `Hexagon`, `Rectangle`, `RoundedRectangle`, `Ellipse`, `Diamond`, `Triangle`, `Underline`, `None`, ou `{"Component": id}` de um componente de detalhe desenhado no tamanho do papel, com os campos dos seus textos preenchidos), sua largura e altura (`width`, `height`; 0 ajusta ao texto), penas, `leader` e `orientation`; `type_set` o altera. O modelo traz um tipo de etiqueta por alvo, no estilo padrão (as etiquetas que as plantas sempre desenharam); um projeto sem tipo de etiqueta para um alvo (um arquivo antigo) desenha as etiquetas nesse mesmo estilo. Cada planta escolhe um tipo de etiqueta por alvo (`view_set` `tag_types`) e ajusta a etiqueta de cada elemento com `tag_set` (`offset` ou `delta`, `rotation`, `type`, `hidden`, `leader`, `reset`; `null` limpa um campo); `tag_get` mostra os ajustes, o tipo e os textos.

```text
{"command": "tag_type_create", "args": {"target": "door", "name": "Porta com tamanho", "style": {"lines": [{"text": "{Mark}", "size": 2.5}, {"text": "{Width} x {Height}", "size": 2}], "symbol": "Rectangle"}}}
{"command": "view_set", "args": {"view": "Térreo", "tag_types": {"door": "Porta com tamanho"}}}
{"command": "tag_set", "args": {"view": "Térreo", "ids": [12, 14], "delta": [0, 0.5], "rotation": 90}}
{"command": "tag_get", "args": {"view": "Térreo", "id": 12}}
```

Eixos e as marcas de cortes e elevações seguem a mesma ideia. O `style` de um tipo de eixo (`type_set` `params.style`) define as pontas com bolha (`ends`: `End`, `Start`, `Both`, `None`; o `ends` do próprio eixo, por `element_set` `params`, vem primeiro), o símbolo da bolha (`symbol`: uma forma, ou um componente de detalhe cujos textos escrevem `{Name}`), `text_size`, `pen`, `color` e `line_pattern`. Cada planta, elevação ou corte move as bolhas de um eixo com `tag_set` (`offset` ou `delta` para a bolha do fim, `start_offset` para a do início, cada uma com um cotovelo até a linha) ou as oculta (`hidden`; a linha fica). Um tipo de marca de vista (`view_mark_type_create`, `kind` `section` ou `elevation` e um `style` parcial) desenha as cabeças dos cortes e as marcas das elevações: o tamanho da cabeça (`head`, em milímetros de papel; 0: 0,5 m no modelo, como as marcas de elevação sempre foram), seu `symbol`, uma linha dividindo-a (`split`), os textos `top`, `bottom` e `label` com `{Number}`, `{Sheet}`, `{Name}` e `{Letter}`, a seta (`arrow`), as cabeças do corte (`heads`), a espessura da linha (`line_weight`) e as cores. Um corte ou uma elevação usa o seu próprio tipo (`view_set` `mark_type`), senão o primeiro do projeto do seu tipo (o modelo traz um de cada, no estilo padrão), senão a marca que as plantas sempre desenharam. `mark_move` move a marca de uma elevação em uma planta (`delta`, `offset` ou `reset`).

Elevações e cortes desenham cada nível como uma linha com uma cabeça na ponta direita. `level_head_set` os ajusta por vista: `hidden` oculta níveis só naquela vista, `heads` põe suas cabeças no fim (`end`), no início (`start`), nas duas pontas (`both`) ou em nenhuma (`none`), `elbow` e `start_elbow` levantam uma cabeça (negativo: abaixam) da linha, em metros na vista, com um cotovelo, e `reset` os devolve ao padrão; `view_get` os lista em `level_heads`. A categoria `level` de `view_graphics_set` oculta todos ou altera suas linhas.

```text
{"command": "type_set", "args": {"type": "Eixo 8 mm", "params": {"style": {"ends": "Both", "symbol": "Hexagon", "text_size": 4}}}}
{"command": "tag_set", "args": {"view": "Térreo", "ids": [30], "offset": [1.5, 0], "start_offset": [-1.5, 0]}}
{"command": "view_mark_type_create", "args": {"kind": "section", "name": "Cabeça grande", "style": {"head": 16, "top": {"text": "{Letter}", "size": 4}}}}
{"command": "view_set", "args": {"view": "Corte A", "mark_type": "Cabeça grande"}}
{"command": "mark_move", "args": {"plan": "Térreo", "view": "Elevação: Sul", "delta": [0, -2]}}
```

### Renderização fotorrealista

Uma vista 3D guarda suas configurações de renderização em `render` (`view_get`), mudadas por `view_set` com um patch de mesclagem: amostras por pixel (`samples`), reflexões da luz (`max_depth`), largura e altura da imagem em pixels (`width`, `height`), `fov_deg`, exposição em stops (`exposure`), mapeamento de tons (`tone_map`: `Filmic` ou `Standard`), o limite contra pontos brilhantes (`clamp`), `clay` (toda superfície opaca num mesmo branco fosco, o vidro mantido, sob o mesmo céu e sol) e o ambiente (`environment`: `sky_strength`, `sun_strength`, `ground_color`, `ground_plane`); `null` as remove. O sol é o da vista (`sun`). `view_render` faz o path tracing da vista no processador e grava a imagem (`.png`, `.jpg`, ou `.exr`/`.hdr` em luz linear) com a câmera da vista, como a vista 3D a mostra; `settings` altera as configurações da vista só para essa renderização, e `region` (`[x, y, largura, altura]` em pixels) renderiza só uma parte da imagem. Retorna o tamanho, as amostras, os segundos e os triângulos, e não muda nada no modelo.

```text
{"command": "view_set", "args": {"view": "{3D}", "render": {"samples": 256, "width": 1920, "height": 1080}}}
{"command": "view_render", "args": {"view": "{3D}", "path": "casa.png"}}
{"command": "view_render", "args": {"view": "{3D}", "path": "maquete.jpg", "settings": {"clay": true, "samples": 64}}}
```

```sh
cargo run -p cc_cli -- -p examples/casa_simples.ccad run view_render view="{3D}" path=saida.png 'settings={"samples":16,"width":320,"height":200}'
```

## Scripts Lua

`ccad lua <arquivo>` executa um script em [Lua 5.4](https://www.lua.org/manual/5.4/) sobre a sessão. O script chama comandos com `ccad.run`, guarda os resultados em variáveis e faz laços, então o id criado num passo pode ser usado no seguinte. Os argumentos (nas mesmas formas do `run`: pares `chave=valor`, um objeto JSON, `@arquivo.json`) ficam em `ccad.args`; o que ele imprime vai para a saída padrão e o que ele retorna é impresso como JSON.

[`examples/small_house.lua`](../examples/small_house.lua) constrói a casa pequena acima, com as medidas dos argumentos:

```lua
local w = ccad.args.width or 8
local d = ccad.args.depth or 6
local corners = { { 0, 0 }, { w, 0 }, { w, d }, { 0, d } }
local outer = {}
for i = 1, 4 do
  outer[i] = ccad.run("wall_create", { start = corners[i], ["end"] = corners[i % 4 + 1], height = 2.8 }).id
end
ccad.run("door_create", { wall = outer[1], offset = w / 4 })
…
```

```sh
ccad --save-as casa.ccad lua examples/small_house.lua width=10 depth=7
```

A tabela `ccad`:

| | |
|---|---|
| `ccad.run(nome, args)` | executa um comando e devolve o resultado como tabela Lua; gera um erro se ele falhar |
| `ccad.try(nome, args)` | o mesmo, mas devolve `nil, err` na falha (`err.code`, `err.message`; veja [Erros](#conceitos)) |
| `ccad.commands()` | os comandos: `name`, `kind`, `group` e `description` de cada um |
| `ccad.describe(nome)` | um comando, com o esquema JSON dos parâmetros em `schema` |
| `ccad.args` | os argumentos do script (também `...`) |
| `ccad.null` | o `null` do JSON (o `nil` do Lua omite o campo) |
| `ccad.array(t)` | marca `t` como lista, para que uma tabela vazia vá como `[]` e não `{}` |
| `ccad.json.encode(v)`, `ccad.json.decode(s)` | texto JSON de e para valores Lua |
| `ccad.version`, `ccad.api` | a versão do ConstructCraft e a da API de plugins (2) |

Argumentos e resultados são JSON: objetos são tabelas com chaves de texto, listas são sequências (`{ 0, 0 }` é um ponto), e `end`, palavra reservada do Lua, se escreve `["end"]`. Um número inteiro calculado com `/` (que em Lua sempre dá um número real) vai como inteiro, então `10 / 2` funciona onde se espera um id ou uma contagem.

**O sandbox.** Os scripts rodam num Lua restrito: as bibliotecas `table`, `string`, `math`, `utf8` e `coroutine` e as funções básicas, sem `io`, `os`, `debug`, `package`, `require`, `load`, `dofile` nem `loadfile`. Um script só chega ao projeto pelos comandos, e a arquivos só pelos comandos que os leem ou gravam. Cada execução tem um limite de 500 milhões de instruções Lua, 60 segundos e 256 MB de memória (as funções de texto contam pelo tamanho do texto, a busca de padrões pelos passos, e `string.rep` e `string.gsub` geram no máximo 16 MB); passando deles, ela para com um erro `script` (capturar esse erro com `pcall` não faz o script continuar). Cada comando que um script executa é um passo de desfazer.

## Grafos de programação visual

Um grafo (`.ccgraph`, JSON) é um conjunto de nós ligados por fios e
executado como um fluxo de dados. **Todo comando do motor é um nó**: seus
parâmetros são as entradas do nó e o resultado é a saída `result` (mais
`element`, o id, nos comandos `*_create`). Os nós internos dão valores de
entrada (`number`, `integer`, `slider`, `boolean`, `text`,
`select_elements`, `elements_of_category`, `levels`), matemática, lógica,
listas, geometria nos valores que os comandos recebem (pontos `[x, y]`,
`line`, `arc`, `circle`, `rectangle`, `rectangle_points`, `polygon_points`,
`polyline_segments`, `divide_line`…), parâmetros de elementos
(`parameter_get`, `parameter_set`) e saídas (`watch`, `output`). Um nó Lua
executa uma função das suas entradas no sandbox (sem acesso ao modelo); ele
retorna as saídas em ordem, ou uma tabela com os nomes delas. `graph_nodes`
lista todos os nós com suas portas e tipos.

- **Listas percorrem os nós.** Um nó que recebe uma lista onde espera um
  valor roda uma vez por item, e suas saídas viram listas (`lacing`:
  `shortest`, `longest` ou `cross`). Uma lista de 4 pontos iniciais e 4
  finais em `wall_create` faz 4 paredes.
- **Rodar de novo atualiza.** Cada elemento criado por um nó fica vinculado
  ao nó e ao item que o fez. A próxima execução o altera no lugar (mesmo id,
  id persistente, marca e comentários), cria os novos e apaga os que não são
  mais feitos; sem mudanças, nada acontece. Uma execução é um passo de
  desfazer. `graph_elements` lista os elementos de um grafo, `graph_release`
  os libera (a próxima execução cria outros).
- **Ordem.** Um nó roda depois dos nós ligados a ele; nós independentes
  rodam na ordem em que foram adicionados (menor id primeiro).
- **Erros são por nó.** Um nó que falha mostra a mensagem, os nós depois
  dele são pulados, os demais rodam, e os elementos dele ficam como estavam.
- Um grafo nunca roda `project_new`, `project_open`, `undo`, `redo`,
  `language_set` nem grafos, e só roda comandos de arquivo quando quem o executa permite: o
  `allow_files` de `graph_run`, `ccad --allow-files graph`, ou a caixa
  *Permitir comandos de arquivo* do editor, que vale até outro grafo ser
  aberto. O arquivo do grafo não pode permitir sozinho (um campo
  `allow_files` nele é ignorado), e um grafo que um plugin executa nunca pode.

```sh
ccad --save-as casa.ccad graph examples/graphs/house.ccgraph width=12 depth=8
ccad run graph_check path=examples/graphs/house.ccgraph
ccad run graph_nodes search=wall
```

Nós de entrada com `name` são as entradas do grafo (`chave=valor` acima,
`inputs` de `graph_run`); nós `output` com `name` são seus resultados. Um
grafo mínimo:

```json
{
  "uid": "6f1c2a0e9b3d4c8a8e2f5a7b1c3d9e02",
  "nodes": [
    { "id": 1, "kind": { "builtin": "range" }, "values": { "end": 15, "step": 5 } },
    { "id": 2, "kind": { "builtin": "point" } },
    { "id": 3, "kind": { "builtin": "point" }, "values": { "y": 4 } },
    { "id": 4, "kind": { "command": "wall_create" }, "values": { "height": 2.8 } }
  ],
  "wires": [
    { "from": [1, "list"], "to": [2, "x"] },
    { "from": [1, "list"], "to": [3, "x"] },
    { "from": [2, "point"], "to": [4, "start"] },
    { "from": [3, "point"], "to": [4, "end"] }
  ]
}
```

## Plugins

Um plugin é uma pasta com um manifesto `plugin.toml` e código Lua que acrescenta comandos ao catálogo. Os comandos dele são listados e executados como os nativos, pelo `ccad run`, por scripts, por scripts Lua e por agentes MCP, e se chamam `<id do plugin>__<comando>` (dois sublinhados, que os nomes nativos nunca têm).

```sh
ccad --plugins examples/plugins commands                # o catálogo, com os comandos dos plugins
ccad --plugins examples/plugins plugins                 # os plugins: permissões, comandos, botões, configurações
ccad --plugins examples/plugins run quick_rooms__rectangle_room width=4 depth=3 name=Escritório door=south
claude mcp add constructcad -- ccad mcp --plugins /caminho/para/plugins
```

`--plugins` recebe a pasta de um plugin ou uma pasta de pastas de plugins, e pode ser repetido. Numa pasta de plugins, a pasta de cada plugin tem o nome do id dele (como as instalações a nomeiam); pastas ocultas (começando com `.`) são ignoradas. Um plugin que não carrega é informado na saída de erro e fica de fora; o que os plugins imprimem também vai para a saída de erro.

### Escrevendo um plugin

[`examples/plugins/quick_rooms`](../examples/plugins/quick_rooms) é um exemplo completo. `plugin.toml`:

```toml
id = "quick_rooms"          # letras minúsculas, dígitos e sublinhados simples, até 32
name = "Quick rooms"
version = "0.2.0"
api = 2                     # a API de plugins para a qual ele foi escrito (1, 2 ou 3, veja abaixo)
description = "Rectangular rooms in one step, and wall totals by type."
authors = ["…"]
license = "GPL-3.0-or-later"
homepage = "https://…"
main = "main.lua"           # o padrão
permissions = []            # ou ["files"], ["tools"] (api 2), ["network"] (api 3), veja abaixo
```

`main.lua` roda uma vez quando o plugin carrega e registra comandos com `ccad.command`:

```lua
local rect = require("lib.rect")   -- lib/rect.lua na pasta do plugin

ccad.command {
  name = "rectangle_room",         -- o comando é quick_rooms__rectangle_room
  title = "Rectangular room",
  description = "Four walls around a rectangle, its floor and its room.",
  kind = "edit",                   -- "query", "edit" (o padrão) ou "file"
  params = {                       -- um esquema JSON dos argumentos
    properties = { width = { type = "number" }, depth = { type = "number" } },
    required = { "width", "depth" },
  },
  run = function(args)
    local p = rect.corners(0, 0, args.width, args.depth)
    for i = 1, 4 do ccad.run("wall_create", { start = p[i], ["end"] = p[i % 4 + 1] }) end
    return { room = ccad.run("room_create", { point = { args.width / 2, args.depth / 2 } }).id }
  end,
}
```

- `run` recebe os argumentos numa tabela e devolve o resultado do comando (uma tabela, um valor ou nada). Argumentos `required` que faltam são recusados antes de ele rodar; `error("mensagem", 0)` faz o comando falhar com essa mensagem.
- **Um passo de desfazer:** quantos comandos um comando de plugin executar, ele é desfeito de uma vez, e quando falha (um erro, um `ccad.run` que falhou, um limite atingido) tudo o que ele fez é desfeito.
- **O que um comando pode executar:** um comando `query` só consulta; um `edit` consulta e edita; comandos que leem ou gravam arquivos (importar, exportar, salvar) precisam da permissão `files` (sem ela, comandos de edição que leem um arquivo indicado por um parâmetro, como `cad_import` ou `fill_pattern_import` com `path`, recusam o caminho; as formas sem arquivo, como `fill_pattern_import` com `text`, continuam funcionando), e só com ela se declara um comando do tipo `file`. Sem `files`, um comando ainda trabalha com os arquivos que os argumentos dele indicam em parâmetros `save_path` e `open_path` ([API 2](#arquivos-sem-a-permissão-files)). Abrir ou criar um projeto, `undo`, `redo` e `language_set` não rodam dentro de um comando de plugin, nem o comando de outro plugin.
- **Essas regras valem para tudo o que um comando executa:** o motor confere cada comando executado enquanto um comando de plugin roda (o escopo dele), também os nós de um grafo que ele roda com `graph_run`; um nó `plugin_data_set` com o id de outro plugin é recusado como um `ccad.run` direto. Os scripts Lua que o usuário roda (`ccad lua`) não têm restrição.
- **O sandbox** é o dos [scripts Lua](#scripts-lua), com o limite contado por comando; cada plugin tem o seu próprio estado Lua, que guarda as variáveis entre um comando e outro. `require("a.b")` carrega `a/b.lua` da pasta do plugin (uma vez; arquivos fora da pasta são recusados), e `ccad.plugin` tem o `id`, o `name` e a `version` dele. `ccad.run` só funciona enquanto um comando roda, não enquanto o plugin carrega.
- **As permissões** são concedidas por quem carrega o plugin: `ccad --plugins` concede o que o manifesto pede, já que você indicou a pasta. `files` deixa os comandos dele trabalharem com qualquer arquivo; `tools` (API 2) deixa que executem os programas externos que o usuário escolhe para as ferramentas dele ([Programas externos](#programas-externos)).

### API de plugins 2: botões, configurações, traduções e dados

Um plugin escrito para `api = 2` (`ccad.api` vale 3 nesta versão, veja a API 3 abaixo) também pode declarar botões da faixa de opções e configurações no manifesto, traduzir os seus textos e dar aos parâmetros dicas para a interface. Manifestos com `api = 1` carregam como antes; botões e configurações são recusados com `api = 1` (`buttons need api = 2`), de modo que o índice nunca oferece um plugin assim a uma versão que só conhece a API 1.

```toml
id = "quick_rooms"
name = "%name%"                  # procurado em locales/<idioma>.toml
version = "0.2.0"
api = 2

[[buttons]]                      # no máximo 64
command = "rectangle_room"       # um comando que o main.lua registra (o nome curto)
label = "%button.room%"          # no máximo 64 caracteres
icon = "icons/room.svg"          # opcional: um SVG na pasta do plugin, no máximo 64 KiB
panel = "%panel%"                # opcional: o painel da faixa (padrão: o nome do plugin)
size = "large"                   # "large" (o padrão) ou "small"
when = "view:floor_plan"         # opcional: quando o botão fica disponível

[settings.default_height]        # no máximo 64; chaves como nomes de comando
type = "number"                  # boolean, number, integer ou string
default = 2.8                    # obrigatório, e tem de caber na configuração
title = "%settings.height%"      # opcional (padrão: a chave legível, "Default height")
description = "Altura das paredes que ele desenha"
ui = "length"                    # opcional, só para números: mostrado nas unidades do projeto
minimum = 2.0                    # números e inteiros; também maximum

[settings.default_door]
type = "string"
default = "none"
enum = ["none", "south", "east", "north", "west"]   # só para textos: os valores permitidos
enum_labels = ["%side.none%", "%side.south%", "%side.east%", "%side.north%", "%side.west%"]   # opcional, um por valor
```

Um botão cujo comando o `main.lua` não registra é um erro de carregamento que cita o comando. **`when`** é uma lista de termos separados por vírgula que precisam valer todos:

| Termo | Vale quando |
|---|---|
| `selection` | há pelo menos um elemento selecionado |
| `selection:one` | há exatamente um elemento selecionado |
| `selection:<categoria>` | há pelo menos um elemento selecionado e todos são da categoria (`wall`, `door`…; os nomes de `categories_list`) |
| `view:<tipo>` | a vista ativa é `floor_plan`, `ceiling_plan`, `three_d`, `elevation`, `section`, `legend`, `drafting`, `schedule`, `diagram` ou `sheet` |

Um termo, categoria ou tipo de vista desconhecido é um erro de carregamento.

**Configurações.** `ccad.settings` tem o valor atual de cada configuração (o valor guardado, ou o padrão): é somente leitura (atribuir a ela gera um erro), existe enquanto o `main.lua` roda e é atualizada antes de cada comando. As configurações são listadas na ordem do manifesto. Os valores são guardados por plugin em `<pasta de plugins>/.settings/<id>.json`; valores guardados que não cabem mais no manifesto (uma chave desconhecida, um tipo errado, um valor fora de `enum`, `minimum` ou `maximum`) são descartados quando o plugin carrega, e um plugin carregado da própria pasta (não de uma pasta de plugins) começa com os padrões. Remover um plugin remove as configurações dele. Pela linha de comando:

```sh
ccad plugins settings ~/plugins quick_rooms                         # as configurações e os valores, em JSON
ccad plugins settings ~/plugins quick_rooms default_height=3 default_door=north
```

Os valores se escrevem como os argumentos `key=value` do `run` (JSON ou texto simples; uma configuração de texto recebe o texto como está), são conferidos com o manifesto e guardados; um valor que não cabe é recusado e nada é gravado. Isso lê só o manifesto e as traduções: o código do plugin não roda.

**Traduções.** Arquivos `locales/<código>.toml` na pasta do plugin, `<código>` um dos idiomas do ConstructCraft (`en`, `pt-BR`, `es`, `fr`, `de`, `it`, `lo`, `pl`, `uk-UA`, `ru`, `zh-TW`, `zh-CN`, `ja`; outros arquivos são ignorados), têm linhas `chave = "texto"` ou tabelas cujas chaves se juntam com `.` (cada arquivo com no máximo 256 KiB; um arquivo que não é TOML de textos é um erro de carregamento):

```toml
# locales/pt-BR.toml
name = "Salas rápidas"
[button]
room = "Sala"                 # a chave button.room
[settings]
height = "Altura padrão"
```

Um texto que é exatamente `%chave%` é procurado quando é mostrado, no idioma atual: o arquivo desse idioma, depois um do mesmo idioma base (`pt` para `pt-BR`), depois `en`; uma chave que não está em nenhum aparece como a própria chave. Isso vale para `name` e `description` do manifesto, `label` e `panel` dos botões, `title`, `description` e `enum_labels` das configurações, `title` e `description` dos comandos, e `title`, `description` e `enum_labels` de cada parâmetro. Trocar o idioma troca os textos na hora, sem recarregar os plugins. Com `api = 1` os textos aparecem como estão escritos.

**Dicas dos parâmetros.** Um parâmetro de `params` pode dizer como a interface o pede; com `api = 2` as dicas são conferidas quando o comando é registrado (uma errada é um erro de carregamento), e passam inalteradas para os clientes MCP:

| `ui` | Tipo do parâmetro | Pede |
|---|---|---|
| `length` | `number` | um comprimento, nas unidades do projeto |
| `element` | `integer` | o id de um elemento (`category` restringe) |
| `elements` | `array` | ids de elementos (`category` restringe) |
| `point` | `array` | um ponto `[x, y]` |
| `level` | `integer` | o id de um nível |
| `view` | `integer` | o id de uma vista |
| `type` | `integer` | o id de um tipo (`category` restringe) |
| `material` | `string` | o nome de um material |
| `save_path` | `string` | um arquivo a gravar (a janela de salvar do aplicativo); o comando pode gravá-lo |
| `open_path` | `string` | um arquivo a ler (a janela de abrir do aplicativo); o comando pode lê-lo |

`category` (o nome de uma categoria) só acompanha `element`, `elements` e `type`; `extensions` (1 a 16 extensões em minúsculas, de 1 a 16 letras ou dígitos, sem o ponto, como `{ "idf", "epw" }`) só `save_path` e `open_path`; `enum_labels` dá um rótulo a cada valor de `enum` (o mesmo número de textos); `title` é um nome curto para o parâmetro.

```lua
params = { properties = {
  width = { type = "number", ui = "length", title = "%param.width%" },
  wall = { type = "integer", ui = "element", category = "wall" },
  side = { type = "string", enum = { "north", "south" }, enum_labels = { "%north%", "%south%" } },
} }
```

**Dados do plugin.** Um plugin guarda os seus próprios dados no projeto, em elementos ou no próprio projeto, salvos no arquivo `.ccad` (os comandos `plugin_data_get`, `plugin_data_set` e `plugin_data_list` da [referência de comandos](commands.md)):

| | |
|---|---|
| `ccad.data.get(elemento)` | o que o plugin guarda no elemento (`nil`: no projeto), ou `ccad.null` |
| `ccad.data.set(elemento, valor)` | guarda `valor` (qualquer JSON, até 256 KiB) ali, no lugar do que havia; `nil` remove |
| `ccad.data.list()` | `{ project = true/false, elements = { ids… } }` |

`ccad.data.set` é uma edição, então um comando `query` não pode chamá-lo, e faz parte do passo único de desfazer do comando. Um comando de plugin alcança só os próprios dados: executar um comando `plugin_data_*` por `ccad.run` ou `ccad.try` com o id de outro plugin é recusado (`a plugin reads and writes only its own data`). Scripts Lua que o usuário roda (`ccad lua`) alcançam os dados de qualquer plugin.

#### Arquivos sem a permissão `files`

Um comando que grava ou lê um arquivo escolhido pelo usuário não precisa da permissão `files`: ele recebe o caminho num parâmetro `save_path` (para gravar) ou `open_path` (para ler) e, enquanto roda, esse caminho, e só ele, fica liberado:

- **Antes de rodar**, cada argumento `save_path`/`open_path` do nível de cima é conferido: um texto, não vazio, com uma das `extensions` do parâmetro quando ele as lista (sem diferenciar maiúsculas: `MODELO.IDF` é um `idf`), um arquivo a ler que existe, um arquivo a gravar numa pasta que existe. Um caminho que falha é recusado (`invalid_arguments`) e o comando não roda. Os caminhos são comparados como arquivos, não como texto: `a/../modelo.idf` e o caminho absoluto de `modelo.idf` são o mesmo arquivo.
- **`ccad.file.read(caminho)`** devolve os bytes de um arquivo `open_path` liberado como texto Lua (no máximo 64 MiB). **`ccad.file.write(caminho, dados)`** grava um arquivo `save_path` liberado; `dados` é um texto ou uma lista de textos, juntados (para saídas grandes, monte uma lista em vez de um texto longo). Um comando grava no máximo 256 MiB ao todo. A direção é conferida: um arquivo `save_path` não pode ser lido, nem um `open_path` gravado, e qualquer outro caminho é recusado (`rejected`). `ccad.file` só existe em plugins, e só funciona enquanto um comando roda.
- **O que é gravado espera o comando terminar bem:** o que `ccad.file.write` recebe fica na memória e só é gravado quando o comando termina sem erro, cada arquivo num arquivo temporário na pasta dele, depois renomeado por cima do destino. Um comando que falha não deixa arquivo nem arquivo pela metade, e um arquivo que já existia fica como estava.
- **As exportações do próprio motor** também usam o caminho liberado: um comando pode rodar `ifc_export`, `gltf_export`, `view_export`, `sheet_export`, `schedule_export_csv`, `analytical_export`, `view_render`, `component_save`, `fill_pattern_export` ou `pipe_series_export` com o seu `save_path`, e importações como `cad_import` com o seu `open_path`; com outro caminho são recusadas, e os demais comandos de arquivo (`project_save`, `ifc_import`…) continuam precisando de `files`. Um arquivo gravado por um comando do motor fica mesmo que o comando do plugin falhe depois (só as mudanças no modelo são desfeitas).
- Um plugin com a permissão `files` trabalha com qualquer arquivo, como antes; `ccad.file` continua lendo e gravando só os caminhos liberados.

Na linha de comando e no MCP o caminho é um argumento comum: `ccad --plugins ~/plugins run energy__export_idf path=modelo.idf`.

#### Exportadores e importadores

Um plugin pode acrescentar itens às listas de Exportar e Importar do aplicativo:

```toml
[[exporters]]                    # no máximo 16
command = "export_idf"           # um comando com exatamente um parâmetro ui = "save_path"
label = "%export.idf%"           # p. ex. "Modelo EnergyPlus (.idf)"; no máximo 64 caracteres

[[importers]]                    # no máximo 16
command = "import_weather"       # um comando com exatamente um parâmetro ui = "open_path"
label = "%import.epw%"
```

Os tipos de arquivo vêm das `extensions` do parâmetro. Um item cujo comando o `main.lua` não registra, ou cujo comando não tem um parâmetro desses ou tem mais de um, é um erro de carga; os dois precisam de `api = 2`. No aplicativo, clicar no item abre a janela de salvar (exportador) ou de abrir (importador) para essas extensões e depois o formulário do comando com o caminho escolhido (ou roda o comando na hora, quando o caminho é o único parâmetro). `ccad plugins` os lista (`exporters`, `importers`: comando, rótulo, parâmetro e extensões).

Os plugins são compartilhados pelo índice da comunidade, [constructcraft-plugins](https://github.com/francisreboucas/constructcraft-plugins): uma entrada por plugin, com de onde baixar cada versão e o checksum dela, enviada por pull request. No aplicativo, *Gerenciar → Plugins* instala, atualiza, desativa e remove plugins ([Plugins](interface.pt-BR.md#plugins)); sem ele, `ccad plugins install <arquivo.zip> <pasta>` instala um arquivo baixado (verificado do mesmo jeito: caminhos seguros, tamanhos e um plugin que carrega) e `ccad plugins remove <id> <pasta>` remove um.

#### Programas externos

Um plugin pode executar um programa instalado no computador, como o simulador EnergyPlus, quando o usuário permite e escolhe qual arquivo de programa é. O programa roda fora da sandbox, com as permissões do usuário, então nada roda sem a aprovação do usuário, e o aplicativo continua funcionando enquanto ele roda.

```toml
permissions = ["tools"]                          # precisa de api = 2

[[tools]]                                        # no máximo 8
id = "energyplus"                                # como um id de plugin; ccad.process.run usa esse nome
name = "EnergyPlus"                              # pode ser "%chave%"
executables = ["energyplus.exe", "energyplus"]   # os nomes de arquivo aceitos do programa
homepage = "https://energyplus.net"              # opcional: onde obtê-lo
version_args = ["--version"]                     # opcional: o que o "Testar" executa
```

`[[tools]]` sem a permissão `tools`, ou com `api = 1`, é um erro de carga; também são arquivos de lote (`.bat`, `.cmd`) e nomes com pastas em `executables`.

**Aprovação.** Duas coisas são escolha do usuário, nunca do plugin:

- **A permissão:** no aplicativo, *Permitir que execute programas externos* na janela Plugins (guardada por plugin, como a permissão `files`); `ccad --plugins` concede o que o manifesto pede, já que você indicou a pasta.
- **O programa:** o caminho de cada ferramenta, escolhido na janela Plugins (*Programas externos*: *Procurar…*, ou *Usar …* para um programa com esse nome encontrado no `PATH`, que nunca é usado sozinho) e guardado em `<pasta de plugins>/.tools/<id do plugin>.json` (`{ "<id da ferramenta>": "<caminho absoluto>" }`; removido com o plugin). Um caminho só vale quando é um arquivo existente cujo nome é um dos `executables` (sem diferença de maiúsculas no Windows). Pela linha de comando:

```sh
ccad plugins tools ~/plugins energy                         # as ferramentas: caminho escolhido, por que não pode rodar, uma sugestão do PATH
ccad plugins tools ~/plugins energy energyplus=/opt/EnergyPlus/energyplus --test   # escolhe e executa version_args
ccad plugins tools ~/plugins energy energyplus=             # esquece
ccad --plugins ~/plugins/energy --tool energy.energyplus=/opt/EnergyPlus/energyplus run energy__simulate
```

`--tool <plugin>.<ferramenta>=<caminho>` (repetível) dá um programa para uma execução, também a um plugin carregado da própria pasta, que não lê caminhos guardados; um caminho que não serve é informado na saída de erro e deixado de fora. `ccad plugins` lista as `tools` de cada plugin (`id`, `name`, `executables`, `homepage`, `path`, `problem`).

**Executando.** Na função `run` de um comando:

```lua
local r = ccad.process.run("energyplus", {
  args = { "-w", "in.epw", "-r", "in.idf" },     -- passados como estão, sem shell
  files = { ["in.idf"] = texto_idf },             -- gravados antes na pasta da execução
  copy = { ["in.epw"] = args.clima },             -- um arquivo dado ao comando para ler (open_path), copiado para lá
  outputs = { "eplusout.csv", "eplustbl.htm", "*.err" },  -- lidos de volta; * casa quaisquer caracteres dentro de um nome
  timeout = 600,                                  -- segundos: padrão 300, no máximo 7200
})
-- r = { code = 0, stdout = "…", stderr = "…", truncated = false, timed_out = false,
--       files = { ["eplusout.csv"] = "…", ["eplusout.err"] = "…" } }
```

- **A pasta da execução:** cada execução ganha uma pasta nova só sua na pasta temporária do sistema (`constructcraft-<plugin>-<aleatório>`), que é a pasta de trabalho do programa; os `files` são gravados e os arquivos de `copy` copiados para ela, as `outputs` encontradas lá são lidas de volta (as não encontradas simplesmente não aparecem em `files`; links não são seguidos), e a pasta é apagada depois, também quando a execução falha, estoura o tempo ou é cancelada.
- **Nomes** em `files`, `copy` e `outputs` são relativos à pasta da execução, nomes simples unidos por `/` (`sub/in.idf`): `..`, caminhos absolutos, unidades, `\`, `:` e nomes de dispositivo do Windows (`CON`, `NUL`, `COM1`…) são recusados.
- **Sem shell:** o programa é iniciado diretamente, e cada argumento chega a ele como escrito: `&`, `;`, `|`, aspas e `$(…)` não significam nada.
- **Ambiente:** o programa recebe do ambiente do aplicativo só `PATH`, `SystemRoot`, `TEMP`, `TMP`, `HOME`, `USERPROFILE`, `LANG` e `LC_*`; nada mais é repassado. A entrada padrão é vazia.
- **Limites:** as entradas (`files` e `copy`) no máximo 256 MiB ao todo; cada saída no máximo 64 MiB e todas as saídas no máximo 256 MiB (além disso, a chamada levanta um erro); saída padrão e de erro guardadas até 1 MiB cada (`truncated = true` quando alguma foi cortada). Ao estourar o tempo o programa é encerrado e `timed_out = true` (sem `code`). Um código de saída diferente de zero é devolvido, não levantado: o plugin decide o que ele significa.
- **Parada:** ao estourar o tempo ou ao cancelar, o programa é encerrado. Programas que ele mesmo iniciou podem continuar rodando: não são usados job objects nem grupos de processos.
- **O que a aprovação significa:** o plugin decide os argumentos e os arquivos de entrada, então escolher um programa de uso geral para uma ferramenta (um shell, um interpretador de scripts) dá ao plugin tudo o que esse programa pode fazer; escolha o programa a que a ferramenta se destina.
- **Erros** são levantados na chamada, como em `ccad.run`, e dizem o que fazer: sem a permissão `tools`, uma ferramenta que o manifesto não declara, nenhum programa escolhido (citando a janela Plugins, `ccad plugins tools` e `--tool`), um programa que não existe ou com outro nome, um nome inválido, um limite, um programa que não pôde iniciar. `pcall` os captura.
- **Onde funciona:** só na própria função `run` de um comando (também dentro de `pcall`), não enquanto o `main.lua` carrega, não em tratadores de eventos, não numa corrotina que o plugin inicia, num módulo sendo carregado por `require` nem numa função chamada por uma função de biblioteca (como o comparador de `table.sort`). Scripts Lua que o usuário roda (`ccad lua`) não têm `ccad.process`.

**O comando espera sem travar o aplicativo.** A função `run` de um comando roda numa corrotina: `ccad.process.run` entrega o programa a uma thread em segundo plano e o comando espera ali. Com o `ccad`, scripts, macros e MCP o comando simplesmente leva o tempo do programa; no aplicativo ele fica pendente: um diálogo mostra o plugin, o programa, o tempo decorrido e a última linha que ele escreveu, com *Cancelar* ([Plugins](interface.pt-BR.md#plugins)). Enquanto espera:

- continua sendo um comando: suas edições antes e depois da espera são um só passo de desfazer, uma falha (ou *Cancelar*) desfaz todas, os arquivos que grava com `ccad.file.write` só são gravados quando termina sem erro, e `model_changed` dispara uma vez, quando termina;
- a sessão só roda consultas: qualquer comando que edita, salva, abre ou exporta, `undo` e `redo`, e outros comandos de plugin são recusados (`a plugin command is running`), e nenhum evento dispara;
- o tempo de espera não conta nos 60 segundos do comando: esse limite é do Lua; seu orçamento de instruções continua através da espera.

Um comando pode executar vários programas, um depois do outro; *Cancelar* encerra o que está rodando e desfaz o comando (`cancelled`). No motor, `Session::set_blocking(false)` deixa esses comandos pendentes (`execute` devolve o código `pending`), `Session::poll_job` continua com eles e devolve o resultado quando terminam, `Session::job` descreve o que está esperando e `Session::cancel_job` o encerra; as sessões bloqueiam por padrão, e comandos rodados por outros comandos (grafos, scripts) e passos de macro são sempre esperados.

#### Eventos

Um plugin pode reagir ao que acontece com o projeto registrando tratadores enquanto o `main.lua` carrega:

```lua
ccad.on("model_changed", function(e)
  local indice = ccad.data.get(nil)
  if indice == ccad.null then indice = { paredes = 0 } end
  indice.paredes = indice.paredes + #e.added - #e.removed   -- (um índice de verdade olharia as categorias)
  ccad.data.set(nil, indice)                                -- os próprios dados: permitido
end)
ccad.on("project_saved", function(e) print("salvo em " .. e.path) end)
```

| Evento | Quando | `e` |
|---|---|---|
| `project_opened` | depois de `project_open`, `project_new` ou de uma importação que substitui o projeto (também Abrir e Novo do aplicativo, mas não Fechar), e uma vez quando os plugins são carregados numa sessão que já tem um projeto, para o plugin começar | `path` (`nil` quando o projeto ainda não tem arquivo), `new` (`true` quando não tem) |
| `project_saving` | antes de `project_save` gravar o arquivo (Salvar e Salvar como do aplicativo, `ccad --save`) | `path` |
| `project_saved` | depois que o arquivo foi gravado | `path` |
| `model_changed` | depois de cada comando que mudou o projeto: `undo` e `redo` também, e um comando de plugin uma vez, quando termina | `command` (o nome do comando: `"wall_create"`, `"undo"`, `"redo"`, `"quick_rooms__rectangle_room"`…), `added`, `modified`, `removed` (ids de elementos, em ordem crescente, no máximo 10 000 em cada, com `truncated = true` quando uma lista foi cortada), e `other = true` quando mudou algo além de elementos (tipos, vistas, níveis, configurações, os dados de plugins do projeto); aí as listas podem estar todas vazias |

- **Registro:** `ccad.on(evento, tratador)` só funciona enquanto o `main.lua` carrega (depois levanta um erro, como `ccad.command`); um nome de evento desconhecido é um erro de carga; um plugin registra no máximo 32 tratadores. Os tratadores rodam plugin por plugin na ordem de carga (numa pasta de plugins, a ordem dos nomes das pastas) e depois na ordem em que foram registrados.
- **Que comandos os disparam:** os que o usuário, um script ou um agente roda, pelo aplicativo, pelo `ccad`, por scripts Lua ou pelo MCP. O que um comando de plugin roda faz parte dele (o comando dispara uma vez, quando termina), e o que um tratador roda nunca dispara nada, então tratadores não entram em laço. Cada passo de uma macro ou script (`ccad script`, `ccad lua`, o editor de macros) é um comando próprio e dispara seus eventos, já que cada um é um passo de desfazer. Comandos que falham, consultas e edições que não mudam nada não disparam nada.
- **O que um tratador pode fazer:** consultar qualquer coisa e manter os próprios dados em dia com `ccad.data.set` (`plugin_data_set` com o próprio id). Qualquer outra edição (`element_set`…) é recusada, assim como arquivos e comandos de outros plugins. Depois de `undo` e `redo` os tratadores só consultam: os dados de plugins fazem parte do histórico do projeto, então desfazer e refazer já os trazem de volta junto com o modelo (e uma edição ali apagaria o que pode ser refeito). O que os tratadores guardam em `model_changed` entra no passo de desfazer do comando que mudou o modelo, então um desfazer leva os dois, e um valor sendo arrastado continua um só passo. O que guardam em `project_opened`, `project_saving` e `project_saved`, que não têm um passo assim, é um passo de desfazer próprio por plugin, com o nome do plugin. Nenhum dos dois é gravado numa macro sendo gravada. Os dados de plugins são salvos com o projeto como o resto: o que é guardado em `project_saving` vai para o arquivo, e guardar algo em `project_opened` ou `project_saved` deixa alterações não salvas.
- **Orçamento:** cada chamada de tratador pode usar no máximo 50 milhões de instruções Lua e 5 segundos (e o limite de memória do plugin).
- **Falhas nunca fazem o comando falhar:** um erro ou um orçamento estourado num tratador vira uma mensagem (`[id] model_changed handler 1: …`, na saída de erro com o `ccad`, na barra de status e no log no aplicativo), e o que aquela chamada guardou é descartado. Um tratador que falha 3 vezes seguidas fica desligado até os plugins carregarem de novo.
- **Custo:** as listas de `model_changed` vêm do mecanismo de desfazer do projeto: enquanto algum plugin escuta esse evento, cada comando anota os elementos que toca como eram antes dele (uma vez cada) e os compara com o que são depois; nenhuma geometria é reconstruída, e nada é anotado enquanto nenhum plugin escuta.

### API de plugins 3: acesso à rede e catálogos de componentes

Um plugin escrito para `api = 3` (`ccad.api` vale 3 nesta versão) também pode baixar dados por HTTPS e oferecer um catálogo de componentes na janela da Biblioteca de Componentes. Manifestos com `api = 1` ou `2` carregam como antes; `network`, `allowed_hosts` e `[[catalogs]]` são recusados abaixo de `api = 3`.

```toml
api = 3
permissions = ["network"]                              # precisa de api = 3
allowed_hosts = ["example.org", "*.cdn.example.org"]   # no máximo 16

[[catalogs]]                                           # no máximo 8
id = "furniture"                                       # como o id de um plugin
label = "Móveis"                                       # o nome da aba; pode ser "%chave%"
description = "Modelos de example.org"                 # opcional, mostrado na barra lateral
```

**A permissão `network`.** Ela precisa de ao menos um host em `allowed_hosts`: nomes de host em minúsculas, ou `*.` e um domínio para os seus subdomínios (não o próprio domínio); sem esquema, porta, caminho, `localhost` ou endereço IP. No aplicativo ela é escolha do usuário, *Permitir acesso à rede* na janela de Plugins (guardada por plugin, desligada de início, mostrando os hosts); `ccad --plugins` concede o que o manifesto pede, já que você indicou a pasta.

**`ccad.http`.** Só são buscadas URLs `https://` cujo host está em `allowed_hosts`, conferido de novo a cada redirecionamento (no máximo 5, para URLs `https://` absolutas ou caminhos no mesmo host); URLs com credenciais ou porta são recusadas, as respostas têm no máximo 64 MiB e as requisições expiram (30 s por padrão, no máximo 120 s). Sem a permissão, cada função é um erro. Uma requisição espera a resposta: num comando, faça poucos downloads e pequenos, e deixe os grandes para um catálogo (abaixo), que roda numa thread própria.

```lua
local body, status = ccad.http.get(url, { max_bytes = 1e6, timeout = 10 })   -- no máximo 16 MiB por padrão
local path = ccad.http.download(url, "models/chair.glb", { sha256 = "…" })  -- para o cache do plugin
local path = ccad.http.cached("models/chair.glb", { sha256 = "…" })         -- nil quando não está lá; sem rede
local text = ccad.http.read_cached("data/index.json", { max_bytes = 1e6 })  -- os bytes, ou nil; sem rede
```

Cada plugin tem a sua pasta de cache (no aplicativo, `plugin-cache/<id do plugin>` na pasta de dados dele; no `ccad`, a pasta de cache do usuário). As chaves são caminhos relativos de letras, dígitos, `.`, `_` e `-`, com no máximo 5 níveis e 160 caracteres, de modo que uma chave nunca sai da pasta. Com `sha256` (64 dígitos hexadecimais), um download só é guardado quando o SHA-256 confere, e um arquivo do cache que deixou de conferir é apagado e lido como ausente; `download` reaproveita um arquivo do cache que confere em vez de buscá-lo de novo. Um comando de plugin pode ler arquivos da sua própria pasta de cache (para passá-los a `gltf_component_load`, por exemplo) sem a permissão `files`.

**Catálogos.** Cada entrada `[[catalogs]]` vira uma aba da janela da Biblioteca de Componentes (ao lado de *Minhas pastas* e da biblioteca online), registrada pelo `main.lua` enquanto ele carrega:

```lua
ccad.catalog {
  id = "furniture",
  load = function()            -- uma vez, quando a aba abre
    return { total = 1200, notice = "…", categories = { { id = "chairs", name = "Cadeiras", count = 140 } },
             colors = { "white", "black" } }
  end,
  search = function(q)         -- q: text, category?, color?, max_width?, max_depth?, max_height? (m), offset, limit
    return { total = 3, items = { { id = "c1", name = "Cadeira", subtitle = "Carvalho", kind = "furniture",
             article = "000.123.45", bytes = 250000, dimensions = { width = 0.45, depth = 0.5, height = 0.8 },
             colors = { "white" }, url = "https://example.org/c1", has_thumbnail = true } } }
  end,
  thumbnail = function(id)     -- opcional: o caminho de uma imagem no cache do plugin
    return ccad.http.download(base .. id .. ".png", "thumbs/" .. id .. ".png")
  end,
  prepare = function(id)       -- quando o usuário carrega o item: baixá-lo e dizer como carregá-lo
    local path = ccad.http.download(base .. id .. ".glb", "models/" .. id .. ".glb", { sha256 = sums[id] })
    return { command = "gltf_component_load", args = { path = path, name = "Cadeira", kind = "furniture" } }
  end,
}
```

As funções do catálogo rodam numa thread própria, de modo que a janela continua respondendo enquanto elas baixam. Uma miniatura precisa ser um arquivo dentro do cache do plugin, com no máximo 4 MiB e 2048 pixels de lado; `prepare` precisa indicar `gltf_component_load`, `component_load` ou `skp_component_load` com um `path` dentro do cache do plugin, senão o aplicativo o recusa. Um catálogo declarado no `plugin.toml` que o `main.lua` não registra é um erro de carga, e `ccad.catalog` chamado depois é um erro. `ccad plugins` lista os `catalogs` e os `allowed_hosts` de cada plugin.

### Desenvolvendo um plugin

**Comece do modelo.** `ccad plugins new <pasta> <id> [name=<nome>]` cria `<pasta>/<id>/` (recusado quando já existe ou o id é inválido) com um plugin que carrega e cujo teste passa: `plugin.toml` (API 2, um botão, uma configuração), `main.lua` (um comando com dicas de interface que lê `ccad.settings` e guarda `ccad.data`), `locales/en.toml` e `locales/pt-BR.toml`, `icons/<id>.svg`, `tests/basic_test.lua`, um `README.md`, e `types/ccad.d.lua` com um `.luarc.json` que aponta o servidor de linguagem Lua para ele.

```sh
ccad plugins new ~/dev room_tools name="Ferramentas de sala"
ccad plugins test ~/dev/room_tools
```

**Tipos.** [`docs/lua/ccad.d.lua`](lua/ccad.d.lua) declara a tabela `ccad` inteira para o [LuaLS](https://luals.github.io/) (a extensão Lua do VS Code e outros editores): `run`, `try`, `commands`, `describe`, `json`, `array`, `null`, `version`, `api`, `plugin`, a especificação de `ccad.command` com seus parâmetros e dicas de interface, `settings`, `data`, `file`, `process` (pedido e resultado), `on` com os nomes e os dados dos eventos, e `ccad.test`. Ponha a pasta dele em `workspace.library` do `.luarc.json` (o modelo já faz isso) para ter completação e verificações. Um teste falha quando a tabela em execução tem um nome que o arquivo não declara, então ele se mantém completo.

**Testes.** `ccad plugins test <pasta do plugin> [--filter <texto>] [--tool <plugin>.<ferramenta>=<caminho>]` executa cada `tests/*_test.lua` do plugin (em ordem de nome) como um script Lua com acesso total, como `ccad lua`, sobre um projeto novo do modelo métrico padrão com o plugin carregado e com o que o manifesto pede (programas de `--tool`, como em `--plugins`). Um arquivo é um teste, seu nível superior; um arquivo que define funções globais com nome `test_*` é um teste por função, cada uma num projeto novo próprio depois que o nível superior do arquivo rodou nele (sua preparação). Os nomes são `<arquivo>` ou `<arquivo>::<função>`; `--filter` executa aqueles cujo nome contém o texto. `ccad.test` tem as verificações, que levantam um erro na linha que as chamou:

| | |
|---|---|
| `ccad.test.eq(atual, esperado, msg)` | valores iguais; tabelas comparadas em profundidade (`expected [1,3], got [1,2]`) |
| `ccad.test.near(atual, esperado, tol, msg)` | números a no máximo `tol` de distância (padrão 1e-9) |
| `ccad.test.fails(fn, padrão)` | `fn` levanta um erro cujo texto contém `padrão` (texto simples ou um padrão Lua); devolve o texto |
| `ccad.test.ok(valor, msg)` | um valor verdadeiro; devolve-o |

```lua
-- tests/rooms_test.lua
local t = ccad.test
function test_a_room_has_four_walls()
  ccad.run("quick_rooms__rectangle_room", { width = 4, depth = 3 })
  t.eq(#ccad.run("elements_list", { category = "wall" }), 4)
end
```

A saída é uma linha por teste, `PASS <nome>` ou `FAIL <nome>: <mensagem>` seguida do que o teste e o plugin imprimiram, e então `<n> passed, <m> failed`. O código de saída é 0 quando todos passaram, 1 quando algum falhou (ou nenhum casou com o filtro), 2 para erros de uso (sem `plugin.toml`, sem testes, um `--tool` que não serve). [`examples/plugins/quick_rooms/tests`](../examples/plugins/quick_rooms/tests) é um exemplo.

**No aplicativo.** *Recarregar plugins*, na janela Plugins, carrega de novo todos os plugins a partir dos arquivos (as permissões ficam; não enquanto um comando de plugin roda). Em *Preferências → Locais de arquivos*, o **Modo de desenvolvedor de plugins** verifica a pasta de plugins e a **Pasta de desenvolvimento** a cada segundo (o tamanho e a data dos arquivos `.lua`, `.toml` e `.svg`) e recarrega os plugins quando algum mudou, com uma mensagem na barra de status; a recarga espera enquanto um comando de plugin roda. A pasta de desenvolvimento (a pasta de um plugin ou uma pasta de plugins) carrega no modo de desenvolvedor como a pasta de plugins, com as mesmas permissões por id; um plugin nas duas carrega da pasta de desenvolvimento, e o console avisa. O **Console de plugins** (*Gerenciar → Plugins*, ou *Console de plugins* na janela Plugins) lista, com o tempo desde que o aplicativo abriu e o plugin, o que os plugins imprimem, seus erros de carregamento, os erros de seus tratadores de eventos e os tratadores desligados, e cada carregamento; mostra um plugin ou todos, *Limpar* o esvazia e *Copiar* copia o que ele mostra; guarda as últimas 2000 entradas ([Plugins](interface.pt-BR.md#plugins)).

## Componentes com script

Uma peça de componente pode ser um **script**: código Lua que declara os parâmetros e os hotspots do componente e monta as peças a partir dos valores deles. O script é puro: ele vê os valores e nada mais (nem comandos, nem arquivos, nem o modelo), então roda uma vez para cada conjunto de valores; mover ou girar um componente já colocado não o roda de novo. [`examples/components/cabinet.lua`](../examples/components/cabinet.lua) é um exemplo completo.

```lua
param.length   { "depth", label = "Profundidade", default = 0.55 }      -- compartilhado pelos elementos do tipo
param.choice   { "handle", { { "bar", "Barra" }, { "none", "Nenhum" } } }
param.length   { "width", label = "Largura", default = 0.8, instance = true }  -- próprio de cada elemento
param.boolean  { "worktop", default = true, instance = true }
param.material { "body", label = "Corpo", instance = true }
hotspot        { "plinth", 0, 0.05, 0, label = "Rodapé", shape = "diamond", color = "#ff8800" }

function geometry(p)
  local pieces = { box { 0, 0, 0, p.width, p.depth, 0.85, material = p.body } }
  if p.worktop then pieces[#pieces + 1] = box { 0, 0, 0.85, p.width, p.depth + 0.02, 0.03 } end
  return pieces
end
```

| Declaração | Valor em `p` |
|---|---|
| `param.length`, `param.angle`, `param.number { id, label=, default=0, instance= }` | um número (metros, graus) |
| `param.boolean { id, default=false }` | `true` ou `false` |
| `param.text { id, default="" }` | um texto |
| `param.choice { id, { {chave, rótulo}, … }, default=primeira chave }` | a chave escolhida |
| `param.material { id, default="" }` | o nome de um material do projeto (vazio: o material da peça) |
| `hotspot { id, x, y, z, label=, shape="circle", color="#00ffff" }` | `{ x, y, z }`, onde o elemento o tem |

As declarações ficam no corpo do script, nunca dentro de `geometry`. Os ids são nomes (letras, dígitos, `_`) e não se repetem; mudar um rótulo não renomeia o parâmetro. Um parâmetro sem `instance = true` é do tipo, e mudá-lo muda todos os elementos do tipo; com ele, cada elemento guarda o seu valor. As formas de hotspot são `circle`, `square`, `triangle`, `diamond` e `triangle_down`; as cores, `#RRGGBB` ou `#RRGGBBAA`. `p` também traz os outros parâmetros do componente, seus planos de referência e, em portas e janelas, `WallThickness`.

`geometry(p)` devolve uma lista de peças (listas de peças são achatadas), no referencial do componente (metros a partir do ponto de inserção, x para a direita, y para trás, z para cima). Cada construtor recebe os valores em ordem ou por nome, além de `name`, `material`, `void`, `in_plan`, `plan_only` e `detail` (uma lista de `coarse`, `medium`, `fine`):

| Construtor | Valores |
|---|---|
| `box` | `x, y, z, width, depth, height` |
| `cylinder` | `x, y, z, radius, height` |
| `extrusion` | `points` (`{x, y}` em planta), `z, height` |
| `profile` | `points` (`{u, v}`, ou `{u, v, radius = r}` para um arco até o próximo ponto), `offset, depth`, `plane = "plan"`, `"front"` ou `"side"` |
| `revolution` | `points, offset, axis, angle` (padrão 360), `plane` |
| `sweep` | `path, profile, offset`, `plane` |
| `blend` | `bottom, top, offset, depth`, `plane` |
| `mesh` | `vertices` (`{x, y, z}`), `faces` (listas de números de vértice a partir de 1, no sentido anti-horário vistas de fora) |

Os scripts rodam no [sandbox](#scripts-lua) com limites menores (20 milhões de instruções, 5 segundos, 64 MB, 10.000 peças, um milhão de pontos), e `math.random` repete os mesmos números. Quando um script falha, o elemento mantém as últimas peças que funcionaram, e o erro, com a linha, aparece em `element_get` (`derived.script_errors`), na paleta de propriedades e em `component_params_get`; corrigir o script ou o valor o recupera.

Comandos:

```sh
ccad run component_create name=Armario 'def={"items": [{"name": "Script", "shape": {"Script": {"source": ""}}}]}'
ccad run component_script_set type=Armario piece=Script source=@examples/components/cabinet.lua
ccad run component_params_get type=Armario
ccad run component_params_set type=Armario name=handle value='"none"'
ccad run component_params_set 'elements=[100, 101]' name=width value=1.2
ccad run component_hotspot_move element=100 name=plinth 'point=[0, 0.08, 0]'
```

- `component_script_set` grava uma peça de script (ou acrescenta uma). Um script que não carrega (erro de sintaxe, declaração inválida) é recusado; as declarações viram os parâmetros e hotspots do componente, os que continuam declarados mantêm os valores quando o tipo ainda serve, e o resultado lista os erros da primeira execução. Um componente tem uma peça de script; seus parâmetros mudam no script, não com `component_param_set` ou `component_param_delete`.
- `component_params_get` mostra os parâmetros e hotspots de um tipo, ou de componentes colocados de um mesmo componente, com `mixed` onde diferem.
- `component_params_set` define o valor de um tipo, ou o valor de instância de vários elementos de uma vez: números, `true`/`false`, textos, a chave de uma opção ou o nome de um material. É um passo de desfazer, e nada muda quando algum elemento recusa o valor.
- `component_hotspot_move` move um hotspot no referencial do componente, ou em coordenadas do projeto com `world=true`.

Um material citado por um parâmetro conta como usado: não pode ser excluído, e renomeá-lo renomeia os valores. A exportação IFC grava as peças que o script fez, como em qualquer componente.

## Macros no aplicativo

*Gerenciar → Macros → Gravar macro* grava as alterações que você fizer até apertar *Parar gravação*; a macro é salva como script (`.ccmacro`, o formato acima). *Executar macro* roda uma macro ou qualquer script no projeto aberto; `ccad script` roda os mesmos arquivos. Com uma pasta de macros definida em *Preferências → Locais de arquivos*, os diálogos de arquivo de macro começam nela e *Executar macro* vira um menu dos arquivos `.ccmacro` dela (mais *Procurar…* para qualquer outro arquivo).

- Só edições são gravadas: abrir, salvar e exportar ficam de fora, então reproduzir nunca sobrescreve arquivos.
- Um valor arrastado nas propriedades é gravado uma vez, com o valor final; alterações feitas num diálogo cancelado não são gravadas.
- Elementos, tipos, vistas e níveis criados durante a gravação são escritos como [referências](#scripts) ao passo que os criou (`{"$step": n}`), de modo que a macro usa o que ela mesma cria a cada execução. O nível da planta em que você grava é escrito como `{"$level": "active"}`: a macro roda no nível da planta ativa quando você a executa (os outros níveis ficam como gravados).
- Os demais passos se referem a elementos pelo id. No projeto em que foi gravada (ou numa cópia dele como estava no início da gravação), a macro reproduz as mesmas alterações; em outro projeto, passos que indicam elementos existentes pelo id podem falhar ou atingir outros elementos.

### Verificar e formatar macros

`macro_check` (com o `text` da macro ou o `path` de um arquivo) lê uma macro sem executá-la e devolve o número de passos e os problemas encontrados, cada um com sua linha, passo e gravidade: linhas que não são passos (JSON inválido, sem `"command"`), comandos que a sessão não tem (comandos de plugins contam), nomes de argumentos que um comando não aceita, argumentos obrigatórios ausentes (um marcador conta como dado), referências `$step` a um passo que não é anterior e valores de `$level` diferentes de `"active"` são erros; chaves de um passo além de `command` e `args`, que a execução ignora, são avisos. `macro_format` reescreve uma macro com um passo por linha na forma compacta em que as macros gravadas são salvas, mantendo comentários e linhas em branco onde estão e as linhas que não são passos como estão.

```sh
ccad run macro_check path=examples/small_house.ccmacro
```

Para a execução passo a passo do aplicativo, `cc_engine::script::Stepper` executa uma macro um passo por vez, cada passo uma etapa de desfazer, e volta sobre o último enquanto nada mais tiver alterado o modelo desde então (uma consulta volta sem desfazer nada); os passos restantes também podem rodar como uma única etapa de desfazer.

## Paleta de comandos no aplicativo

**Ctrl+Shift+P** (ou a lupa no fim das abas do ribbon) abre a paleta de comandos. Ela encontra comandos da interface, vistas para abrir ou criar e os comandos do motor pelo nome, pela descrição ou pelo atalho, sem diferenciar acentos nem maiúsculas. Comandos do motor abrem um formulário com os argumentos em JSON, já com os campos obrigatórios; **Executar** (Ctrl+Enter) os roda no projeto aberto, como um passo de desfazer, e mostra o resultado. Os comandos de arquivo (`project_open`, `ifc_export`…) ficam na aba *Arquivo*.

A dica de cada botão do ribbon mostra o comando do motor que ele executa, para que o que você faz à mão possa virar um script.

## Servidor MCP

`ccad mcp` é um servidor [Model Context Protocol](https://modelcontextprotocol.io/) na entrada/saída padrão. Cada comando é uma ferramenta com o mesmo nome, descrição e esquema de entrada; as consultas são marcadas como somente leitura. O servidor mantém uma sessão, então um agente abre ou cria um projeto, trabalha nele ao longo de várias chamadas e o salva.

**Claude Code:**

```sh
claude mcp add constructcad -- ccad mcp
# ou já com um projeto aberto
claude mcp add constructcad -- ccad mcp -p /caminho/para/casa.ccad
```

**Claude Desktop e outros clientes** (`claude_desktop_config.json` ou as configurações MCP do cliente; use o caminho completo do `ccad`):

```json
{
  "mcpServers": {
    "constructcad": { "command": "/caminho/para/ccad", "args": ["mcp", "--lang", "pt-BR"] }
  }
}
```

No Windows, o `ccad.exe` é instalado ao lado do aplicativo, por padrão em `C:\Program Files\ConstructCraft` (no JSON as barras invertidas são dobradas):

```json
{
  "mcpServers": {
    "constructcad": { "command": "C:\\Program Files\\ConstructCraft\\ccad.exe", "args": ["mcp", "--lang", "pt-BR"] }
  }
}
```

A opção *Add ccad to the PATH* do instalador (marcada por padrão) permite que os clientes achem só `ccad`, mas apenas clientes abertos depois da instalação enxergam o PATH novo; o caminho completo sempre funciona. Apontado para o próprio aplicativo (`constructcraft mcp` ou `constructcraft --mcp`), o cliente recebe o mesmo servidor: o aplicativo executa o `ccad` ao lado dele, na mesma entrada e saída padrão.

Exemplos de pedidos que um agente resolve com essas ferramentas: *"abra casa.ccad, liste as portas sem marca e numere-as"*, *"importe modelo.ifc e diga o volume das paredes de concreto"*, *"crie um pavimento superior 3 m acima do térreo com as mesmas paredes externas"*, *"exporte a folha A101 em PDF"*.

Observações:

- Os caminhos são da máquina onde o `ccad` roda.
- Erros voltam como resultado da ferramenta com `isError` e o objeto de erro, para que o modelo corrija a chamada.
- O servidor fala as revisões 2025-06-18, 2025-03-26 e 2024-11-05 do protocolo e é implementado em `crates/cc_cli/src/mcp.rs` (JSON-RPC 2.0, uma mensagem por linha).

## Para desenvolvedores: criar um comando

Toda funcionalidade de usuário nasce como comando do motor; o aplicativo chama o comando em vez de alterar o documento.

1. Escreva a estrutura de parâmetros em `crates/cc_engine/src/commands/<área>.rs`: `#[derive(Debug, Deserialize, Serialize, JsonSchema)]`, `#[serde(deny_unknown_fields)]`, um comentário de documentação (a descrição do comando) e um em cada campo (a descrição do parâmetro; um teste falha sem eles). O código e os comentários são em inglês.
2. Implemente `Command` com `NAME` (`snake_case`), `KIND` e `run`. Resolva referências com `refs`, altere o modelo dentro de `commands::edit` (um passo de desfazer, recusado se o resultado precisar de reparo) e devolva JSON (`created(s, id)` para elementos). Use mensagens `t!` para erros que pessoas vão ler.
3. Registre-o no seu grupo em `registry.rs`.
4. Teste-o em `crates/cc_engine/tests/commands.rs` por `Session::execute`.
5. Gere a referência de novo: `cargo run -p cc_cli -- reference > docs/commands.md` (um teste falha enquanto ela estiver desatualizada).
6. No aplicativo, chame-o com `Session::run(Comando { … })`, `actions::run_command` ou os auxiliares de `engine_edit` (use `Session::run_merged` para valores arrastados). Isso também o torna gravável em macros.

Comandos acrescentados durante a execução implementam `cc_engine::Extension` (`crates/cc_engine/src/extension.rs`) e são postos na sessão com `Session::set_extension`; `cc_plugins::PluginHost` é a dos plugins Lua. `Session::commands`, `describe` e `kind_of` cobrem os dois, e `Session::execute` roda um comando de extensão como um passo de desfazer (`Document::squash_since`).

Os detalhes de arquitetura estão em [AGENTS.md](../AGENTS.md).
