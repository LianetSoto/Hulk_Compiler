# Compilador HULK - Reporte Técnico

> **Este reporte presenta un resumen ejecutivo** de la arquitectura del compilador y las extensiones implementadas.  
> Para un análisis detallado de las decisiones de diseño del lenguaje HULK, una discusión más exhaustiva de cada fase del compilador y comparativas con otros lenguajes, consulte el **informe técnico completo** disponible en el repositorio (`informe.pdf`).


## Introducción

HULK (Havana University Language for Kompiers) es un lenguaje de programación estáticamente tipado, orientado a expresiones y orientado a objetos, desarrollado en la Universidad de La Habana como parte del curso de Compilación y Lenguajes de Programación. Este documento reporta el diseño, implementación y extensión de un compilador completo para el lenguaje HULK, desarrollado como proyecto semestral.

La arquitectura del compilador sigue un pipeline tradicional de etapas múltiples: análisis léxico, análisis sintáctico, análisis semántico y generación de código. Cada etapa transforma la representación del programa desde el código fuente hasta una representación intermedia, culminando en LLVM IR que puede compilarse a código máquina nativo.


## Extensiones del Lenguaje

La especificación original de HULK proporciona una base sólida pero carece de ciertas características comunes en lenguajes de programación modernos. Identificamos e implementamos dos extensiones significativas que abordan limitaciones en la expresividad del lenguaje: polimorfismo paramétrico para funciones y tipado estructural mediante protocolos.

### Polimorfismo Paramétrico en Funciones

#### Motivación

En la especificación original de HULK, cada función tiene un único tipo estático y no puede reutilizarse con argumentos de diferentes tipos, incluso cuando su implementación es completamente independiente de los tipos de datos que manipula. Considérese la función identidad:

```hulk
function id(x) => x;
```

Conceptualmente, esta función debería aceptar cualquier valor y devolver ese mismo valor. Sin embargo, sin parámetros de tipo, la función debe vincularse a un único tipo concreto durante la compilación. Esto impide reutilizar la misma implementación con diferentes tipos de argumentos, forzando a los programadores a escribir múltiples versiones equivalentes de la misma función.

El siguiente programa expresa una intención natural desde la perspectiva del programador:

```hulk
print(id(42));
print(id("Hola"));
```

Sin embargo, estas dos llamadas no pueden coexistir con una única definición de `id` en el lenguaje original. Esta restricción reduce las capacidades de abstracción del lenguaje y complica la construcción de bibliotecas reutilizables.

#### Implementación

Para superar esta limitación, introdujimos soporte para funciones genéricas, permitiendo que la misma definición se utilice con diferentes tipos de argumentos sin duplicar la implementación.

La genericidad puede declararse explícitamente mediante el parámetro de tipo reservado `T`:

```hulk
function id(x: T): T => x;
```

Esto requirió extender la gramática del lenguaje para permitir que las anotaciones de tipo acepten, además de los tipos concretos (Number, String, Boolean y tipos definidos por el usuario), el identificador reservado `T`.

Sin embargo, la anotación explícita es opcional: el programador puede omitir completamente las anotaciones de tipo:

```hulk
function id(x) => x;
```

Ambas definiciones son equivalentes, haciendo válido el ejemplo motivador:

```hulk
print(id(42));
print(id("Hola"));
print(id(true));
```

La extensión mantiene la filosofía de diseño del lenguaje. Así como HULK permite omitir anotaciones de tipo para variables, funciones o atributos, la genericidad puede expresarse implícita o explícitamente, dejando que los programadores decidan cuándo documentar la interfaz de una función y cuándo confiar en la inferencia del compilador.

#### Consideraciones Semánticas

Las funciones genéricas no pueden utilizarse indiscriminadamente con cualquier tipo. Aunque la función sea genérica, los tipos con los que puede instanciarse siguen determinados por las operaciones realizadas en su implementación. Si un tipo particular no satisface dichas operaciones, el compilador detecta la incompatibilidad durante la comprobación de tipos y rechaza la llamada antes de la ejecución.

Esta extensión acerca HULK a lenguajes como Java, C#, C++ y Rust, todos ellos con soporte para polimorfismo paramétrico. Sin embargo, el enfoque difiere en un aspecto importante: mientras estos lenguajes exigen que la genericidad se declare explícitamente en la interfaz de la función, HULK mantiene la anotación como opcional, permitiendo que el compilador infiera el comportamiento genérico cuando sea posible.

### Protocolos

#### Motivación

El sistema de tipos original de HULK es puramente nominal: dos tipos solo son compatibles si pertenecen a la misma jerarquía de herencia. Como resultado, dos clases que ofrecen un comportamiento idéntico siguen siendo incompatibles si no comparten un ancestro común.

Esta restricción dificulta la reutilización de código y obliga a diseñar jerarquías de clases con anticipación, incluso cuando solo se desea expresar que diferentes tipos proporcionan el mismo conjunto de operaciones.

Considérese un escenario donde tanto `Circle` como `Square` definen un método `area()`. Sin un supertipo común que declare este método, las operaciones que requieren cálculo de área no pueden aceptar ambos tipos polimórficamente, aunque la interfaz sea idéntica.

#### Implementación

Para superar esta limitación, añadimos soporte para protocolos, introduciendo un mecanismo de tipado estructural que complementa el sistema nominal de clases existente. Un tipo conforma a un protocolo cuando proporciona los métodos requeridos por el protocolo, sin necesidad de declarar explícitamente su implementación.

Este modelo se asemeja mucho al enfoque de Go y, en cierta medida, al tipado estructural de TypeScript, donde la conformidad se determina implícitamente a partir de la estructura del tipo. En contraste, lenguajes como Java, C#, Rust y Swift requieren declaraciones explícitas de implementación de interfaces o traits.

**Sintaxis de Declaración de Protocolos:**

```hulk
protocol Shape {
    area(): Number;
}
```

**Conformidad Implícita:**

```hulk
type Circle(radius: Number) {
    method area() => 3.14159 * radius * radius;
}

type Square(side: Number) {
    method area() => side * side;
}

function printArea(shape: Shape) {
    print(shape.area());
}
```

Tanto `Circle` como `Square` conforman implícitamente al protocolo `Shape` porque proporcionan el método `area()` requerido. La función `printArea` puede aceptar cualquiera de los dos tipos sin modificación, logrando polimorfismo sin herencia.

#### Decisiones de Diseño

Los protocolos pueden extender otros protocolos, creando una estructura jerárquica de requisitos. Esto permite capacidades de abstracción más finas mientras se mantiene la simplicidad de la implementación implícita.

Las reglas de varianza se diseñaron cuidadosamente: los tipos de retorno son covariantes (un subtipo puede devolver un tipo más específico), y los tipos de parámetros son contravariantes (un subtipo puede aceptar un tipo más general). Esto garantiza la seguridad de tipos mientras se preserva la flexibilidad.

La principal ventaja del enfoque adoptado es que promueve la reutilización y el desacoplamiento. Un protocolo puede definirse independientemente de los tipos existentes, y cualquier tipo que proporcione los métodos apropiados conformará automáticamente, incluso si fue desarrollado con anterioridad. Esto reduce significativamente el acoplamiento entre componentes y permite una organización de código más flexible.

Como contrapartida, la implementación implícita hace menos evidente qué protocolos satisface un tipo con solo observar su declaración. Mientras que los lenguajes con implementación explícita hacen esta información parte de la interfaz pública, HULK requiere deducir la conformidad a protocolos a partir de los métodos definidos por el tipo.

### Iterables
#### Motivación
En el diseño original de HULK, el bucle `for` estaba restringido a la función predefinida `range`, lo que limitaba su utilidad a la iteración sobre secuencias numéricas. Esta decisión simplificaba la implementación inicial, pero suponía una seria limitación desde el punto de vista de la expresividad del lenguaje. En lenguajes modernos, el `bucle` for se concibe como una construcción universal capaz de recorrer cualquier tipo de datos que exponga una interfaz de iteración.

Sin un mecanismo abstracto de iteración, el desarrollador se ve forzado a escribir bucles `while` manuales para cada tipo de colección, acoplando la lógica de control a la estructura concreta y dificultando la reutilización de código. Además, se pierde la oportunidad de definir iteraciones perezosas o personalizadas —por ejemplo, recorridos con filtros o transformaciones— que son comunes en el desarrollo moderno.

La extensión de iterables aborda estas carencias, permitiendo que cualquier tipo definido por el usuario pueda ser utilizado en un bucle `for`, siempre que implemente un conjunto mínimo de operaciones. De este modo, HULK se alinea con la filosofía de lenguajes que priorizan la composición y la abstracción, como Python, Rust o Java.

#### Implementación
Para dotar al lenguaje de un mecanismo de iteración uniforme, se definieron dos protocolos fundamentales, inyectados como parte del núcleo del lenguaje:


    protocol Iterable {
        next(): Boolean;   // Avanza al siguiente elemento; devuelve true si hay elemento
        current(): Object; // Devuelve el elemento actual
    }

El protocolo `Iterable` establece el contrato mínimo para un recorrido de una sola pasada. Cualquier tipo que proporcione estos métodos con las firmas adecuadas conforma automáticamente a `Iterable`, gracias al sistema de protocolos de HULK (tipado estructural).

Sin embargo, este protocolo solo permite una única iteración: una vez que el iterador ha llegado al final, no puede reiniciarse. Para aquellos tipos que necesitan ser recorridos múltiples veces (por ejemplo, una lista o un rango reutilizable), se definió un segundo protocolo:


    protocol Enumerable {
        iter(): Iterable; // Devuelve un nuevo iterador para la colección
    }
Un tipo que implementa `Enumerable` garantiza que cada invocación a iter() devuelve un iterador independiente y en su estado inicial, permitiendo bucles anidados o múltiples recorridos sobre los mismos datos sin interferencias.

Soporte para `range` como iterable predefinido
La función predefinida `range(inicio, fin` devuelve una instancia del tipo interno `_Range`, que se inyecta durante el análisis semántico. Este tipo posee los atributos `min`, `max` y `current`, e implementa el protocolo `Iterable` mediante los métodos:


    next(): Boolean => (self.current := self.current + 1) < self.max;
    current(): Number => self.current;
De este modo, `range` se integra perfectamente en el mecanismo de iteración, sin necesidad de un tratamiento especial en el backend más allá de la creación del objeto. Además, al implementar Iterable y no Enumerable, cada iteración sobre un rango produce un nuevo iterador implícitamente, lo que permite múltiples usos de range en distintos bucles.

## Arquitectura del Compilador

El compilador está implementado en Rust y organizado como un pipeline de etapas sucesivas. Cada etapa consume la representación producida por la etapa anterior y genera una nueva representación que acerca el programa a su forma ejecutable final.

### Análisis Léxico

El analizador léxico transforma el código fuente en una secuencia de tokens consumida por el analizador sintáctico. La implementación sigue la arquitectura clásica basada en expresiones regulares y autómatas finitos.

#### Especificación de Tokens

Los elementos léxicos se representan mediante el tipo enumerado `Token`, agrupando palabras reservadas, operadores, símbolos de puntuación, literales, identificadores y las nuevas construcciones añadidas durante la extensión del lenguaje. Los operadores y símbolos fijos se describen mediante expresiones regulares literales, mientras que las palabras reservadas se distinguen después del reconocimiento a partir del lexema.

#### Construcción del Autómata

Cada expresión regular es procesada inicialmente por un analizador que construye su árbol sintáctico, representando las operaciones fundamentales de expresiones regulares: concatenación, unión, clausura de Kleene y símbolos literales.

A partir de este árbol, el algoritmo de Thompson construye un autómata finito no determinista (NFA) con un único estado inicial y un único estado final. Los autómatas para todos los patrones se unifican en un único NFA mediante un nuevo estado inicial conectado por transiciones épsilon.

El autómata resultante se transforma en un autómata finito determinista (DFA) utilizando el algoritmo de construcción de subconjuntos. Cada estado del DFA representa un conjunto de estados del NFA y almacena, cuando corresponde, el token asociado al estado de aceptación de mayor prioridad. Esta conversión permite el reconocimiento de tokens mediante simulación completamente determinista.

#### Estrategia de Tokenización

Durante la compilación, el código fuente se recorre secuencialmente simulando el DFA construido. Desde cada posición de entrada, el analizador avanza mientras exista una transición válida, registrando el último estado de aceptación alcanzado. Cuando no es posible continuar, se genera el token correspondiente al prefijo reconocido más largo, y el proceso continúa desde la siguiente posición.

Este enfoque implementa la estrategia de "coincidencia más larga" (longest match), resolviendo correctamente conflictos entre operadores que comparten prefijos, como `=` y `==`, `:` y `:=`, o `@` y `@@`. Cuando múltiples patrones reconocen el mismo lexema con igual longitud, se respeta la prioridad establecida durante la construcción del autómata.

#### Reconocimiento Manual de Literales

Aunque el DFA reconoce la mayoría de los tokens del lenguaje, ciertos lexemas requieren un tratamiento específico más convenientemente manejado mediante código procedural que mediante expresiones regulares. Antes de iniciar la simulación del autómata, el lexer detecta secuencialmente:

1. Espacios en blanco y comentarios, que se descartan
2. Literales numéricos enteros y reales
3. Cadenas de caracteres, incluyendo el procesamiento de secuencias de escape
4. Identificadores y palabras reservadas, distinguiendo posteriormente entre palabras clave e identificadores definidos por el usuario

### Análisis Sintáctico

El analizador sintáctico verifica que la secuencia de tokens conforma a la gramática del lenguaje y construye el Árbol de Sintaxis Abstracta (AST) cuando la entrada es válida.

#### Gramática y Generación del Parser

La sintaxis de HULK se especifica mediante una gramática libre de contexto donde cada producción describe cómo se combinan los diferentes elementos del lenguaje para formar construcciones válidas. El parser se genera automáticamente a partir de esta gramática utilizando LALRPOP.

Elegimos LALRPOP por varias razones:

1. **Manejo natural de recursividad izquierda**: HULK tiene una amplia variedad de operadores con diferentes precedencias y asociatividades. LALR(1) maneja reglas como `AddSubExpr + AddSubExpr + MulDivExpr` sin problemas, respetando la asociatividad izquierda directamente.

2. **Potencia expresiva**: LALR(1) cubre prácticamente todas las construcciones sintácticas de lenguajes de programación modernos (clases, herencia, protocolos, macros, bucles). Representa el punto óptimo entre la potencia del LR(1) canónico y la compacidad del SLR.

3. **Integración con herramientas**: LALRPOP se integra perfectamente con el ecosistema de Rust, genera código limpio y seguro, y proporciona excelentes mensajes de error para el desarrollo iterativo.

4. **Escalabilidad**: HULK es un lenguaje incremental que añade características complejas. Los analizadores LR escalan mejor a medida que la gramática crece, ya que no requieren reestructuraciones profundas para acomodar nuevas reglas.

#### Transformaciones de Desazúcar

La implementación de la gramática incluye desazúcar de ciertas construcciones del lenguaje. En lugar de introducir nodos específicos en el AST para cada forma sintáctica, algunas construcciones se transforman en otras más fundamentales ya presentes en el núcleo del lenguaje.

**Desazúcar del Elif:**

La cláusula `elif` se transforma en expresiones `if-else` anidadas. El parser recolecta todas las ramas `elif` y, partiendo desde la rama `else` final, envuelve cada `elif` en un nodo `IfExpr` estándar. Esto crea una estructura donde la primera condición aparece en el nivel más externo y las condiciones subsiguientes solo se evalúan si las anteriores son falsas.

### Árbol de Sintaxis Abstracta

El AST sirve como representación interna del programa, eliminando detalles puramente sintácticos mientras preserva la información necesaria para el análisis semántico y la generación de código.

#### Estructura General

El nodo `Program` actúa como raíz, agrupando diferentes categorías de definiciones: tipos, protocolos, funciones y la expresión principal que constituye el punto de entrada del programa. Esta organización refleja la estructura global de HULK.

#### Representación de Expresiones

Todas las construcciones evaluables se representan mediante el enumerado `Expr`. Cada variante corresponde a una forma de expresión diferente, como operaciones binarias, llamadas a funciones, condicionales, bloques, bucles, creación de objetos o acceso a atributos.

#### Representación de Declaraciones

Las diferentes declaraciones se representan mediante nodos especializados: `FunctionDef`, `TypeDef` y `ProtocolDef`. Cada uno encapsula información específica de la entidad que representa, evitando estructuras excesivamente generales y simplificando el procesamiento posterior.

#### Anotaciones de Tipo

Para los elementos donde el lenguaje permite anotaciones de tipo (parámetros, atributos o tipos de retorno), el AST distingue entre el tipo especificado por el programador (`ty_annotation`) y el tipo finalmente asignado por el compilador (`ty`). Esta separación preserva la información original del programa mientras registra los resultados de la inferencia.

#### Patrón Visitor

Todos los nodos del AST implementan la interfaz `Node`, definiendo el método `accept` correspondiente al patrón de diseño Visitor. Cada fase del compilador implementa su propio visitante para recorrer el árbol y realizar tareas específicas, desacoplando la estructura del AST de las operaciones realizadas sobre él.

### Análisis Semántico

El análisis semántico verifica la corrección lógica del programa y enriquece el AST con la información de tipos necesaria para las etapas posteriores.

#### Inferencia y Comprobación de Tipos

El sistema de tipos se modela mediante el enumerado `HulkType`, representando tanto tipos concretos del lenguaje (Number, String, Boolean, Object y clases definidas por el usuario) como construcciones auxiliares necesarias para la inferencia. Las variables de tipo (`Var`) actúan como marcadores para tipos desconocidos, y el tipo absorbente `Error` propaga fallos sin generar nuevos diagnósticos para expresiones inválidas.

El motor de unificación mantiene una tabla de sustituciones asociando variables de tipo con los tipos concretos a los que se han vinculado, y una tabla de restricciones limitando la posible resolución de cada variable. Durante el recorrido del AST, cada operación impone ecuaciones sobre los tipos de sus operandos.

Actualmente se soportan dos restricciones semánticas:

1. **StringOrNumber**: Impuesta por los operadores de concatenación `@` y `@@`, requiriendo que las variables se vinculen exclusivamente a String o Number, ya que ambos son convertibles a cadena.

2. **ConformsToProtocol**: Asociada con parámetros anotados con un protocolo, requiriendo que cualquier tipo vinculado a la variable implemente estructuralmente todos los métodos del protocolo respetando las reglas de varianza.

#### Manejo de Protocolos e Iterables
El sistema de tipos incluye soporte para protocolos como mecanismo de abstracción estructural. Durante la fase de registro de símbolos, el `TypeChecker` almacena cada definición de protocolo en una tabla junto con la firma de sus métodos y la referencia a su protocolo padre. La conformidad de una clase a un protocolo se verifica comprobando que la clase implemente todos los métodos requeridos con las firmas adecuadas, respetando las reglas de varianza.

Un caso particularmente relevante es el de los iterables, que responden a la sintaxis `T*` —azúcar sintáctico para el tipo `Iterable(T)`. El compilador registra internamente el protocolo base `Iterable`. Durante el análisis de un tipo definido por el usuario, si la clase implementa `next(): Boolean` y `current(): Object`, se extrae el tipo de retorno de `current()` y se registra en un mapa que asocia ese tipo de elemento con el nombre de la clase.

Cuando el analizador encuentra una expresión o anotación de tipo `Iterable(T)`, aplica una función que, si existe una clase registrada cuyo método `current()` devuelva exactamente `T`, la sustituye por esa clase; en caso contrario, genera automáticamente un protocolo especializado `Iterable_T` que extiende `Iterable` y redefine `current(): T`. Esta transformación se aplica en todos los lugares donde se asigna un tipo a un nodo del AST, de modo que el código generado en fases posteriores nunca recibe un tipo genérico Iterable sin resolver.

El bucle `for` se procesa siguiendo dos pasos. Primero, se evalúa el tipo de la expresión iterable. Si ese tipo conforma al protocolo `Enumerable` (que exige el método `iter(): Iterable`), el iterable se reescribe como una llamada a `iterable.iter()`, transformando así un for sobre una colección múltiple en un for sobre el iterador que esta produce. En segundo lugar, se verifica que el iterable resultante sea una clase concreta que implemente los métodos next() y `current()`; el tipo de los elementos se obtiene del retorno de `current()` y se declara una variable en el ámbito del bucle con ese tipo.

Finalmente, el aplanamiento de la jerarquía no necesita conocer la existencia de iterables ni protocolos, pues todos los tipos Iterable ya han sido sustituidos por clases o protocolos concretos en el AST. La fase de aplanamiento trabaja exclusivamente con las estructuras FlattenedType construidas a partir de las clases, y los métodos que implementan iteradores o enumeradores se tratan como cualquier otro método.

#### Aplanamiento de la Jerarquía de Tipos

Antes de que el backend pueda generar código, la jerarquía de clases debe transformarse en una representación plana que elimine las dependencias de herencia. Para cada tipo definido por el usuario, se construye una estructura `FlattenedType` con tres componentes esenciales:

1. Una lista ordenada de todos los atributos que un objeto de esa clase tendrá en memoria (atributos heredados primero, seguidos de los propios)
2. Un vector de métodos efectivos con sus índices para la Tabla Virtual
3. La lista de parámetros que el constructor de la clase acepta realmente

Durante este proceso, se resuelven las relaciones de herencia. Si un hijo sobrescribe un método del padre, la nueva implementación ocupa exactamente la misma posición en la VTable, garantizando la compatibilidad con el polimorfismo. Los métodos nuevos reciben índices incrementales al final de la tabla.

#### Gestión de Errores

El análisis semántico se ejecuta mediante un proceso de múltiples fases, acumulando errores en un vector en lugar de detenerse en el primero. Cuando una expresión es inválida, se registra un diagnóstico y se le asigna el tipo `Error`, que actúa como absorbedor. Esto evita cascadas de diagnósticos redundantes y permite a los programadores recibir un informe completo de los problemas en una sola ejecución.

### Monomorfización

Después del análisis semántico, todas las expresiones poseen información de tipos inferida. Sin embargo, las funciones marcadas como genéricas aún contienen variables de tipo en sus firmas y anotaciones del cuerpo. Dado que LLVM IR requiere tipos concretos para generar código, se necesita un pase adicional para eliminar la genericidad antes de la traducción final.

#### Descripción del Proceso

El pase de monomorfización transforma cada función genérica en una o más versiones concretas especializadas para los tipos realmente utilizados en el programa. El proceso se implementa mediante el patrón Visitor.

El núcleo del proceso opera en los siguientes pasos:

1. Los argumentos se visitan recursivamente, asegurando que las llamadas genéricas anidadas se especialicen antes de procesar la llamada actual
2. Se determina el estado genérico de la función invocada a partir del atributo establecido durante el análisis semántico
3. Si es genérica, se recolectan los tipos concretos de los argumentos y se genera un nombre único para la especialización mediante "name mangling"
4. Si la especialización no existe, se clona la definición original y se reemplazan todas las variables de tipo por tipos concretos
5. La versión especializada se almacena para reutilización futura, y la llamada se reescribe para referenciar la nueva versión concreta
6. Las definiciones genéricas se eliminan del AST después de completar el procesamiento

#### Manejo de Recursión

Para evitar la generación infinita de especializaciones en presencia de funciones recursivas genéricas, el pase mantiene un conjunto de especializaciones en progreso. Si se detecta un ciclo (intento de instanciar una especialización que ya se está procesando), el compilador emite un error de compilación.

El resultado final es un AST completamente monomórfico que puede traducirse sin que el backend necesite conocer la existencia de la genericidad.

### Generación de Código

La etapa final del compilador traduce el AST a LLVM Intermediate Representation (LLVM IR). En este punto, toda la información necesaria se ha resuelto, incluyendo los tipos de las expresiones, la vinculación de identificadores y la estructura completa de la jerarquía de herencia.

#### Infraestructura LLVM

La generación de código se centraliza en la estructura `LlvmCodeGen`, responsable de mantener el estado durante la traducción. Esta estructura encapsula los principales componentes proporcionados por LLVM: el `Context` que gestiona tipos y objetos compartidos, el `Module` que almacena todas las definiciones globales, y el `Builder` utilizado para insertar secuencialmente instrucciones LLVM IR dentro del bloque básico activo.

La traducción se realiza mediante el patrón Visitor, comenzando la lógica de generación en el nodo raíz `Program`, que coordina las diferentes fases necesarias para construir el módulo LLVM.

#### Declaración de Funciones y Métodos

El backend declara todas las funciones y métodos antes de generar sus respectivos cuerpos. Esta separación es necesaria porque HULK permite llamadas independientemente del orden de definición.

Durante esta primera pasada, solo se construyen las firmas LLVM de cada función, especificando nombre, tipo de retorno y tipos de parámetros. Para los métodos, se añade implícitamente un primer parámetro correspondiente al objeto receptor (`self`), representado como un puntero a la estructura del tipo. Cada método se registra con un nombre completamente calificado de la forma `Tipo.metodo`, evitando conflictos entre métodos con el mismo nombre definidos en clases diferentes.

#### Representación de Objetos

Cada tipo definido por el usuario se traduce a una estructura (struct) de LLVM. La disposición de los campos se obtiene a partir de la representación aplanada construida durante el análisis semántico. Como primer campo de toda estructura se incluye un puntero a la Tabla Virtual, con los atributos ocupando las posiciones restantes.

#### Tablas Virtuales

Cada tipo tiene una única VTable global, compartida por todas sus instancias. Esta estructura centraliza toda la información dinámica de tipos:

1. Un identificador entero único (`type_id`) asignado durante el análisis semántico
2. Un puntero a la VTable del padre
3. Un arreglo de punteros a funciones correspondiente a los métodos efectivos del tipo

Los dos primeros campos implementan el mecanismo de identificación dinámica de tipos. El identificador permite reconocer el tipo concreto de un objeto en tiempo de ejecución, mientras que el enlace a la VTable del padre posibilita recorrer la jerarquía de herencia cuando es necesario. Esta información se utiliza para implementar las operaciones `is` y `as`.

El arreglo de métodos implementa el mecanismo de despacho dinámico. Durante el análisis semántico, cada método recibe una posición fija dentro de la VTable. Cuando un método es sobrescrito en una subclase, solo se reemplaza el puntero almacenado.

#### Traducción del bucle for
El bucle for de HULK se transpila a una combinación de let y while que sigue el protocolo Iterable. Para una expresión for (x in iterable) cuerpo, la transformación equivale a:


    let _iter = iterable in
        while (_iter.next())
            let x = _iter.current() in
                cuerpo
Esta traducción se realiza directamente en el backend durante la generación de código, sin necesidad de modificar el AST. El bucle resultante invoca dinámicamente los métodos `next()` y `current()` a través de la tabla de métodos virtuales (VTable), lo que permite iterar cualquier objeto que implemente el protocolo Iterable, independientemente de su tipo concreto. El valor de retorno del for es el de la última ejecución del cuerpo, igual que en un while convencional.

#### Soporte para el protocolo Enumerable
Para que una colección pueda recorrerse múltiples veces, HULK define el protocolo Enumerable, que añade el método `iter()`. Cuando un objeto implementa este protocolo, el for ya no lo itera directamente, sino que primero obtiene un nuevo iterador. La especificación del lenguaje establece la siguiente transformación:


    let enumerable = <expresión> in
        let iterable = enumerable.iter() in
            while (iterable.next())
                let x = iterable.current() in
                    cuerpo
El backend incorpora esta extensión mediante una comprobación previa al bucle: si el tipo del objeto posee el método iter, emite una llamada virtual a `iter()` y el iterador resultante reemplaza al objeto original.

#### La función range como iterable predefinido
La función range(inicio, fin) es una construcción built‑in que devuelve un objeto iterable. Para integrarla con el mecanismo de despacho virtual, el analizador semántico inyecta automáticamente el tipo interno _Range, cuyos atributos (min, max, current) y métodos (next(), current()) cumplen el protocolo Iterable. La llamada a range se traduce a la creación de una instancia de _Range, inicializando sus campos con los argumentos recibidos.

Este diseño permite que range se beneficie del mismo mecanismo de iteración que cualquier otra clase definida por el usuario, sin requerir un tratamiento especial en el backend más allá de la construcción inicial del objeto.

### Gestión de Errores

El compilador sigue un modelo de detección temprana: cada etapa se ejecuta secuencialmente y verifica que no se hayan producido errores antes de pasar a la siguiente. Si una fase encuentra problemas, el proceso se detiene y los errores se reportan. Cada error incluye la ubicación exacta en el código fuente (línea y columna), una categoría que identifica la fase de origen, y un mensaje descriptivo que explica la causa del fallo.



## Limitaciones

- El sistema de tipos no incluye polimorfismo paramétrico para tipos (genéricos en clases), solo para funciones.
- No se soportan iteradores ni colecciones nativas (vectores, listas), lo que limita la expresividad en programas más allá de ejemplos académicos.
- La inferencia de tipos es relativamente simple y no cubre todos los casos posibles; en algunos escenarios el programador debe proporcionar anotaciones explícitas.

## Conclusión

Este proyecto ha implementado un compilador funcional para HULK, extendiendo el lenguaje con polimorfismo paramétrico iterables y protocolos. La arquitectura en pipeline, basada en Rust y LLVM, ha demostrado ser robusta y modular, facilitando la incorporación de nuevas características. El compilador maneja correctamente el núcleo del lenguaje y las extensiones, generando código nativo eficiente. La experiencia ha permitido aplicar los conocimientos teóricos de compilación en un entorno práctico, y deja abierta la puerta a futuras mejoras que enriquecerán aún más el lenguaje.