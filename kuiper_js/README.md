# Kuiper JS

Javascript bindings for the kuiper programming language, using webassembly.

## Usage

```typescript
import { compile_expression, KuiperError, KuiperExpression } from '@cognite/kuiper_js';

const expr = compile_expression("input.test + 5", ["input"]);
const result = expr.run({ "test": 3 }); // Returns the value 8
```

The input to expressions may be any plain, JSON-serializable javascript object, meaning it should not have cycles.
