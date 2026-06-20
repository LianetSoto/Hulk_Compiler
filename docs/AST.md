Documentación del AST (Árbol Sintáctico Abstracto) para HULK
### 1. Concepto y motivación del AST

El Árbol Sintáctico Abstracto (AST) es la representación intermedia principal que el front-end de un compilador construye a partir del análisis sintáctico. A diferencia del árbol de análisis concreto (que refleja exactamente las producciones de la gramática), el AST abstrae los detalles sintácticos irrelevantes para las fases posteriores, como paréntesis, separadores, y ciertos no terminales auxiliares. El resultado es una estructura de datos jerárquica que captura la esencia semántica del programa: cada nodo corresponde a una construcción del lenguaje (expresión, instrucción, declaración, etc.), y sus hijos representan los componentes significativos de esa construcción.

En nuestro compilador, el AST de HULK está definido en el módulo ast, y se compone de una serie de estructuras (struct) y enumeraciones (enum) que modelan todas las construcciones del lenguaje: expresiones aritméticas y lógicas, variables, asignaciones, condicionales, bucles, funciones, tipos, protocolos, etc.

### 2. Fundamentos teóricos: Traducción orientada por la sintaxis

La construcción del AST es una aplicación directa de la traducción orientada por la sintaxis. Cada producción de la gramática tiene asociada una acción semántica que construye un nodo del AST. Estas acciones se ejecutan durante el análisis sintáctico (en nuestro caso, durante las reducciones del parser LALR) y producen un árbol cuyos nodos son objetos de las clases correspondientes.

En la teoría de atributos, los nodos del AST pueden tener atributos sintetizados y heredados. En nuestra implementación, los atributos más relevantes son:

    Tipo (ty: Option<HulkType>): almacena el tipo inferido de la expresión o construcción. Este atributo se sintetiza durante el análisis semántico (type checking).

    Span (span: Span): guarda la posición en el código fuente, útil para reportar errores.

    Nombre y parámetros para funciones y métodos.

Además, utilizamos el patrón Visitor para recorrer el AST y realizar diferentes operaciones: impresión, análisis semántico, generación de código, etc. La ventaja del visitor es que separa el algoritmo de la estructura del árbol, facilitando la adición de nuevas operaciones sin modificar las clases de los nodos.

### 3. Estructura del AST de HULK
#### 3.1. Enumeración principal: Expr

El núcleo del AST es el enum Expr, que agrupa todas las posibles expresiones de HULK. Cada variante incluye una estructura con los campos específicos. Algunas variantes notables:

    Number(NumberExpr), String(StringExpr), Bool(BoolExpr), Const(ConstExpr): literales.

    Variable(VariableExpr): referencia a una variable.

    BinaryOp(BinaryOpExpr): operación binaria (aritmética, lógica, relacional, concatenación).

    UnaryOp(UnaryOpExpr): operación unaria (!, -).

    Call(CallExpr): llamada a función global.

    MethodCall(MethodCallExpr): llamada a método (incluye el receptor).

    Let(LetExpr): expresión let ... in ... para declarar variables locales.

    DestructiveAssign(DestructiveAssignExpr): asignación destructiva (:=).

    If(IfExpr), While(WhileExpr): estructuras de control.

    Block(BlockExpr): bloque de expresiones entre { }.

    New(NewExpr): creación de instancia de tipo.

    AttributeAccess(AttributeAccessExpr): acceso a atributo.

    SelfExpr(SelfExpr), Base(BaseExpr): referencias a self y base.

Cada estructura asociada contiene campos para los hijos (expresiones, identificadores, listas) y los atributos comunes (span y ty).

#### 3.2. Definiciones de nivel superior

Además de las expresiones, el AST incluye nodos para las declaraciones de nivel superior:

    Program: contiene vectores de tipos, protocolos, funciones y la expresión principal.

    TypeDef: definición de un tipo (atributos, métodos, herencia).

    ProtocolDef: definición de un protocolo (métodos sin implementación).

    FunctionDef: definición de función global (nombre, parámetros, cuerpo, anotaciones de tipo).

    Method: definición de método dentro de un tipo.

    Attribute: definición de atributo con su inicialización.

Estas estructuras también incluyen campos para el span y el ty (en el caso de funciones y métodos, el tipo de retorno).

#### 3.3. Atributos y patrones de diseño

El AST de HULK utiliza:

    Atributos sintetizados: el tipo (ty) se calcula durante el análisis semántico mediante una pasada de inferencia y comprobación de tipos.

    Visitor: la trait Visitor (definida en ast/visitor.rs) permite implementar el recorrido del AST sin acoplar la lógica a las clases. Por ejemplo, el PrettyPrinter visita el AST para generar una representación textual.

    Inmutabilidad: los nodos del AST son inmutables después de su construcción; las fases posteriores (como el type checking) pueden crear nuevos ASTs o agregar información mediante nuevos campos (como ty).

### 4. Desazucarización del for en el AST

Aunque el parser ya realiza la desazucarización del for, es útil entender cómo quedaría representado en el AST si no se hiciera. En nuestra implementación, el parser produce directamente un Let con un While y una asignación destructiva. Esto significa que el análisis semántico y la generación de código no necesitan conocer la existencia del for, ya que el AST ya contiene solo construcciones primitivas.

Esta estrategia se alinea con el principio de reducción de complejidad que se menciona en el manual de HULK: al desazucarizar en el parser, se reduce el número de casos que deben manejar las fases posteriores, lo que simplifica el código y reduce la posibilidad de errores.

### 5. Recorrido del AST y aplicaciones

El AST no es un fin en sí mismo, sino un punto de partida para las fases posteriores del compilador. Una vez construido, el árbol se recorre sistemáticamente para extraer información y transformar el programa. El primer recorrido importante lo realiza el analizador semántico (TypeChecker), que visita cada nodo para inferir tipos, verificar que las anotaciones de tipo sean consistentes, resolver sobrecargas de operadores y detectar errores como el uso de variables no declaradas o la aplicación incorrecta de operadores. Este análisis es fundamental para garantizar la corrección del programa antes de generar código.

Posteriormente, el AST se utiliza como entrada para la generación de código intermedio. En este paso, un recorrido transforma la estructura jerárquica en una secuencia lineal de instrucciones de tres direcciones, siguiendo el formato BANNER IR. Cada construcción de alto nivel (bucles, condicionales, llamadas a métodos) se descompone en operaciones primitivas que una máquina virtual o un compilador de bajo nivel puede ejecutar. Este recorrido es especialmente sensible a la estructura del AST, ya que debe preservar el orden de ejecución y las dependencias entre expresiones.

Además, el AST puede recorrerse con fines de depuración o visualización. El PrettyPrinter es un ejemplo claro: visita el árbol y produce una representación textual indentada que muestra la jerarquía de nodos, útil para entender cómo se interpretó el programa fuente. Este mismo mecanismo podría extenderse para implementar optimizaciones locales, transformaciones sintácticas (como la expansión de macros) o incluso para generar documentación automática a partir del código.

### 6. Ventajas del AST

La principal fortaleza del AST reside en su capacidad para abstraer los detalles superficiales de la sintaxis. Al eliminar elementos como paréntesis, punto y coma innecesarios o no terminales auxiliares, el árbol se centra en la semántica del programa, lo que facilita su manipulación por parte de las fases posteriores. Esta abstracción también unifica construcciones que son sintácticamente distintas pero semánticamente equivalentes, como los múltiples elif que se aplanan en un solo nodo If con varias ramas.

Otra ventaja crucial es la facilidad de manipulación. Al ser una estructura de datos en memoria (un grafo acíclico de nodos), recorrerlo, buscar patrones o transformarlo es sencillo y eficiente. Esto permite implementar pases de análisis y generación de código de forma modular, sin tener que reanalizar el texto fuente original.

Por último, el AST es inherentemente extensible. Añadir una nueva construcción al lenguaje (por ejemplo, un nuevo tipo de bucle o una expresión de ámbito) solo requiere agregar una nueva variante al enum Expr y actualizar el visitor para que maneje esa variante en las operaciones pertinentes. Esta separación entre la estructura y las operaciones, facilitada por el patrón Visitor, reduce el acoplamiento y hace que el compilador sea más mantenible a medida que el lenguaje evoluciona.


### 7. Conclusión

El AST de HULK es el pilar central del front-end. Su diseño sigue los principios de la traducción orientada por la sintaxis, utilizando atributos y el patrón Visitor para separar las fases de análisis y generación. La decisión de desazucarizar el for en el parser, y no en fases posteriores, demuestra una aplicación práctica de la teoría de compiladores para simplificar la arquitectura del compilador. El resultado es un AST limpio, manejable y adecuado para soportar todas las operaciones necesarias hasta la generación de código final.
