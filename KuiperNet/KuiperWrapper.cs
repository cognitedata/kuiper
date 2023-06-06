using System;
using System.Linq;
using System.Runtime.InteropServices;

namespace Cognite.Kuiper {
    public class Kuiper : IDisposable {
        private readonly KuiperExpression _expression;

        public Kuiper(string data, string[] inputs) {
            // var inputsConverted = inputs.Select(inp => GCHandle.Alloc(InputStringWrapper.New(inp).Context, GCHandleType.Pinned)).ToArray();
            var inputsConverted = inputs.Select(inp => InputStringWrapper.New(inp).Context).ToArray();

            var result = Interop.compile_expression(data, inputsConverted);
            
            var err = result.error.ToNullable();
            if (err.HasValue) {
                Console.WriteLine($"{err.Value}");
                using var errValue = ((GCHandle)err.Value).Target as KuiperError;
                throw new CompileException(errValue);
            }
            var expr = (GCHandle)result.result.ToNullable().Value;
            _expression = expr.Target as KuiperExpression;
        }

        public void Dispose()
        {
            _expression.Dispose();
        }

        public string Execute(params string[] inputs) {
            var inputsConverted = inputs.Select(inp => InputStringWrapper.New(inp).Context).ToArray();
            var handle = GCHandle.Alloc(inputsConverted, GCHandleType.Pinned);
            var result = _expression.Execute(new SliceInputStringWrapper(handle, (ulong)inputsConverted.Length));
            var err = result.error.ToNullable();
            if (err != null) {
                using var errValue = ((GCHandle)err.Value).Target as KuiperError;
                throw new ExecuteException(errValue);
            }
            var expr = (GCHandle)result.result.ToNullable().Value;

            handle.Free();
            return (expr.Target as KuiperExpressionResult).Data();
        }
    }

    public class CompileException : System.Exception {
        public CompileException(KuiperError err) : base(err.Error()) {}
    }

    public class ExecuteException : System.Exception {
        public ExecuteException(KuiperError err) : base(err.Error()) {}
    }
}