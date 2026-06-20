## Documentación del Parser (Analizador Sintáctico) para HULK
### 1. Introducción: El papel del analizador sintáctico

El analizador sintáctico (parser) es la segunda fase del front-end de un compilador. Su función principal es recibir el flujo de tokens producido por el analizador léxico y determinar si esa secuencia puede ser generada por la gramática del lenguaje fuente. En caso afirmativo, construye una representación intermedia que refleja la estructura jerárquica del programa. En la práctica, esta representación suele ser un árbol sintáctico abstracto (AST), que descarta detalles sintácticos irrelevantes para las fases posteriores.

La teoría de análisis sintáctico se divide en dos grandes familias: descendente (top-down) y ascendente (bottom-up). Nuestro compilador HULK emplea un analizador ascendente de la familia LR, concretamente el método LALR(1), implementado mediante la herramienta LALRPOP. Esta elección no es casual: los analizadores LR (y en particular LALR) son los más potentes y eficientes para lenguajes de programación reales, y LALRPOP proporciona una forma cómoda y segura de generar el parser en Rust.

### 2. Fundamentos teóricos
#### 2.1. Gramáticas libres de contexto

La sintaxis de HULK se especifica mediante una gramática libre de contexto (GLC). Una GLC consta de:

    Un conjunto de terminales (los tokens).

    Un conjunto de no terminales (categorías sintácticas).

    Un símbolo inicial.

    Un conjunto de producciones de la forma A→αA→α, donde AA es un no terminal y αα es una secuencia de terminales y no terminales.

Por ejemplo, en nuestro parser, la producción para una expresión de suma se define como:

    AddSubExpr: Box<Expr> = {
        <l:AddSubExpr> "+" <r:MulDivExpr> => ...
    }

Esta regla indica que un AddSubExpr puede ser otro AddSubExpr seguido de + y un MulDivExpr. Esta definición recursiva por la izquierda captura la asociatividad izquierda de los operadores aritméticos, algo que los analizadores LR manejan de forma natural.
#### 2.2. Analizadores LL vs LR: diferencias fundamentales

Los analizadores sintácticos se dividen en dos grandes familias, definidas por la dirección en la que construyen el árbol de análisis y el tipo de derivación que utilizan:

- Analizadores LL (descendentes o top-down): 
Construyen el árbol desde la raíz hacia las hojas. Parten del símbolo inicial y aplican producciones para expandir los no terminales hasta que coinciden con la entrada. Utilizan una derivación por la izquierda y, en cada paso, miran el siguiente token de entrada (lookahead) para decidir qué producción aplicar (análisis predictivo). El nombre LL(1) significa:

    - L Left-to-right: escaneo de izquierda a derecha.

    - L Leftmost derivation: derivación por la izquierda.

    - (1) Lookahead 1: un símbolo de anticipación.

    Son fáciles de implementar a mano (descenso recursivo) y producen mensajes de error muy precisos. Sin embargo, no pueden manejar recursividad por la izquierda (ej. E → E + T), lo que obliga a reescribir la gramática mediante eliminación de recursión izquierda y factorización de prefijos comunes. 

- Analizadores LR (ascendentes o bottom-up): Construyen el árbol desde las hojas hacia la raíz. Van leyendo la entrada de izquierda a derecha y, cuando reconocen el cuerpo completo de una producción en la cima de la pila, reducen ese cuerpo al no terminal correspondiente (operación de shift-reduce). Utilizan una derivación por la derecha en orden inverso. El nombre LR significa:

   - L Left-to-right: escaneo de izquierda a derecha.

   - R Rightmost derivation: derivación por la derecha (construida en orden inverso).

    Manejan la recursividad por la izquierda de forma natural, reconocen una clase de gramáticas mucho más amplia y detectan errores sintácticos tan pronto como es posible (propiedad de prefijo viable). Sin embargo, son difíciles de implementar a mano; requieren generadores automáticos como Yacc, Bison o LALRPOP.

La diferencia fundamental es que los analizadores LL expanden no terminales para que coincidan con la entrada (predicción), mientras que los LR reducen terminales y no terminales para reconstruir el símbolo inicial (reconocimiento). Esta distinción se refleja en el hecho de que los LR retrasan la decisión de qué producción aplicar hasta haber visto suficiente entrada.

#### 2.3. Variantes del análisis LR

El algoritmo LR se basa en autómatas finitos deterministas sobre los items (producciones con un punto que indica la posición del análisis) y una tabla de acciones (desplazar, reducir, aceptar, error). Existen varias variantes, cada una con un equilibrio distinto entre potencia y tamaño de la tabla:

- SLR (Simple LR): Usa los conjuntos SIGUIENTE (Follow) de los no terminales para resolver conflictos. Es el más sencillo, pero también el menos preciso, ya que puede aceptar reducciones en contextos donde no son válidas.

-  LR(1) canónico: Usa items con contexto de un símbolo de anticipación (lookahead). Es el más potente, pero produce tablas enormes (cientos o miles de estados), lo que lo hace poco práctico para lenguajes grandes.

-  LALR(1) (Look-Ahead LR): Combina estados LR(1) que tienen el mismo kernel (corazón, es decir, los items sin contar el lookahead). Mantiene la potencia del LR(1) para la mayoría de los lenguajes de programación, pero con un número de estados similar al SLR. Es el método estándar en herramientas como Yacc, Bison y LALRPOP. La construcción de la tabla LALR se basa en la colección canónica de items LR(0) y la información de SIGUIENTE para resolver conflictos, como se describe en el PDF de LR parsing.

#### 2.4. ¿Por qué LALR(1) y LALRPOP para HULK?

La elección de LALR(1) y LALRPOP responde a razones tanto teóricas como prácticas:

- Manejo natural de la recursividad izquierda: HULK tiene una amplia variedad de operadores con distintas precedencias y asociatividades. Con un analizador LL, la gramática de expresiones tendría que reescribirse extensamente, haciéndola menos legible y más propensa a errores. LALR(1) maneja reglas como AddSubExpr → AddSubExpr + MulDivExpr sin problemas, respetando la asociatividad izquierda de forma directa.

-   Potencia expresiva: LALR(1) cubre prácticamente todas las construcciones sintácticas de los lenguajes de programación modernos (clases, herencia, protocolos, macros, bucles). Es el "punto óptimo" entre la potencia del LR(1) canónico y la compacidad del SLR.

-  Herramienta adecuada (LALRPOP): LALRPOP está escrito en Rust y se integra perfectamente con el ecosistema del proyecto. Genera código limpio y seguro, con excelentes mensajes de error que facilitan el desarrollo iterativo. Permite incrustar acciones semánticas (construcción del AST) directamente en las reglas gramaticales.

-  Resolución de conflictos y precedencia: LALR(1) resuelve conflictos de desplazamiento-reducción de forma predecible. En LALRPOP, la precedencia se define estructuralmente (anidando reglas), y el generador elige por defecto el desplazamiento para dar mayor prioridad a los operadores más internos, que es exactamente el comportamiento deseado.

-  Escalabilidad: HULK es un lenguaje incremental que añade características complejas (inferencia de tipos, macros). Un analizador LR escala mejor a medida que la gramática crece, ya que no requiere reestructuraciones profundas para acomodar nuevas reglas, a diferencia de los analizadores LL que necesitan factorización izquierda constante.

### 3. Implementación del Parser con LALRPOP
#### 3.1. Estructura general

El archivo grammar.lalrpop contiene la gramática completa del lenguaje HULK. La sección extern declara los tokens que vienen del analizador léxico, incluyendo literales, operadores y palabras clave. Luego se definen las reglas de producción, organizadas de menor a mayor nivel sintáctico: primero los tipos, funciones, protocolos, y finalmente las expresiones con sus distintas precedencias.

La gramática está diseñada para ser no ambigua y fácilmente analizable por LALRPOP. Por ejemplo, las reglas de expresiones utilizan recursión izquierda, que LALRPOP maneja correctamente generando un analizador LR que utiliza reducciones para construir el árbol.

#### 3.2. Manejo de tipos

El parser reconoce tipos básicos (Number, String, Boolean, Object) y tipos definidos por el usuario (identificadores). Esto se refleja en la regla Type: HulkType = ..., que produce un valor del enum HulkType. También se permite anotar tipos en parámetros, variables, métodos, etc., mediante la sintaxis : Type.

#### 3.3. Desazucarización del for en el parser

Una de las decisiones de diseño más interesantes es que el bucle for no es una construcción primitiva, sino que se desazucariza (desugars) en el parser a una combinación de let, while y asignación destructiva. Esto simplifica enormemente las fases posteriores (análisis semántico y generación de código), ya que solo deben manejar un conjunto reducido de construcciones básicas.

La regla para ForExpr construye un AST equivalente a:
    
    let var = from in
    while (var < to) {
        body;
        var := var + 1;
    }

Es decir, se genera un nodo Let que introduce la variable de iteración con el valor inicial from; el cuerpo del let es un While que verifica la condición var < to; dentro del bucle se ejecuta el body original y luego se incrementa var mediante una asignación destructiva. Esta desazucarización es posible porque HULK restringe el for a rangos numéricos ascendentes (con range).

#### 3.4. Resolución de conflictos y precedencia

La gramática de expresiones está diseñada para reflejar la precedencia estándar: ^ tiene mayor precedencia que * y /, que a su vez tienen mayor que + y -, y los operadores relacionales y lógicos están en niveles inferiores. Esto se logra mediante la jerarquía de reglas LogicalOrExpr → LogicalAndExpr → ComparisonExpr → AddSubExpr → MulDivExpr → PowerExpr → UnaryExpr → PostfixExpr → AtomicExpr.

LALRPOP, al ser un generador LALR(1), maneja gramáticas con recursión izquierda sin problemas. Los posibles conflictos de desplazamiento-reducción (por ejemplo, entre + y * en una expresión como a + b * c) se resuelven a favor del desplazamiento, lo que en la práctica corresponde a la mayor precedencia de *. De forma similar, la asociatividad izquierda se obtiene al reducir cuando se encuentra el mismo operador.

### 4. Conclusión

El parser de HULK, implementado con LALRPOP, demuestra la aplicación práctica de los principios teóricos del análisis sintáctico ascendente LR. La elección de LALR(1) proporciona un equilibrio entre potencia y eficiencia, permitiendo manejar la gramática completa del lenguaje sin conflictos. La desazucarización del for en el propio parser es un ejemplo de cómo las decisiones de diseño pueden simplificar las fases posteriores, alineándose con la filosofía de construir un compilador modular y mantenible. Al comprender la diferencia fundamental entre los enfoques LL y LR, y las ventajas específicas de LALR(1), queda justificada la elección tecnológica que sustenta todo el front-end del compilador HULK.
