using System;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;

namespace Cognite.Kuiper
{
    /// <summary>
    /// Exception thrown by the Kuiper mapping language.
    /// </summary>
    public class KuiperException : Exception
    {
        /// <summary>
        /// Index of the first _byte_ affected by the error.
        ///
        /// Start and end may both be 0 if there is no known range.
        /// </summary>
        public ulong Start { get; }
        /// <summary>
        /// Index of the last _byte_ affected by the error plus one.
        ///
        /// Start and end may both be 0 if there is no known range.
        /// </summary>
        public ulong End { get; }

        public KuiperException(string message, ulong start, ulong end) : base(message)
        {
            Start = start;
            End = end;
        }
    }

    internal struct KuiperError
    {
#pragma warning disable CS0649 // These fields are assigned in external code.
        public unsafe byte* error;
        public bool is_error;
        public ulong start;
        public ulong end;
#pragma warning restore CS0649
    }

    internal struct CompileResult
    {
#pragma warning disable CS0649 // These fields are assigned in external code.
        public KuiperError error;
        public IntPtr result;
#pragma warning restore CS0649
    }

    internal struct TransformResult
    {
#pragma warning disable CS0649 // These fields are assigned in external code.
        public KuiperError error;
        public unsafe byte* result;
#pragma warning restore CS0649
    }
    internal static class KuiperInterop
    {
        public const string NativeLib = "kuiper_interop";

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "compile_expression")]
        public unsafe static extern CompileResult* compile_expression(byte* data, byte** inputs, UIntPtr len);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "destroy_compile_result")]
        public unsafe static extern void destroy_compile_result(CompileResult* data);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "get_expression_from_compile_result")]
        public unsafe static extern IntPtr get_expression_from_compile_result(CompileResult* result);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "destroy_transform_result")]
        public unsafe static extern void destroy_transform_result(TransformResult* result);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "run_expression")]
        public unsafe static extern TransformResult* run_expression(byte** data, UIntPtr len, IntPtr expression);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "destroy_expression")]
        public unsafe static extern void destroy_expression(IntPtr data);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "expression_to_string")]
        public unsafe static extern byte* expression_to_string(IntPtr expression);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "destroy_string")]
        public unsafe static extern void destroy_string(byte* data);
    }

    public class KuiperExpression : IDisposable
    {
        private IntPtr _expression;

        /// <summary>
        /// Compile a kuiper expression.
        ///
        /// This will throw a `KuiperException` if compilation failed.
        /// </summary>
        /// <param name="expression">Expression code</param>
        /// <param name="inputs">A list of available input arguments</param>
        public KuiperExpression(string expression, params string[] inputs)
        {
            unsafe
            {
                var rawExpression = Encoding.UTF8.GetBytes(expression + char.MinValue);
                var rawInputs = inputs.Select(inp => Encoding.UTF8.GetBytes(inp + char.MinValue)).ToArray();

                GCHandle[] pinnedInputs = new GCHandle[rawInputs.Length];
                byte*[] inputPtrs = new byte*[rawInputs.Length];

                for (int i = 0; i < rawInputs.Length; i++)
                {
                    pinnedInputs[i] = GCHandle.Alloc(rawInputs[i], GCHandleType.Pinned);
                    inputPtrs[i] = (byte*)pinnedInputs[i].AddrOfPinnedObject();
                }

                var pinnedExpression = GCHandle.Alloc(rawExpression, GCHandleType.Pinned);
                var expressionPtr = (byte*)pinnedExpression.AddrOfPinnedObject();

                KuiperException exc = null;
                if (inputPtrs.Length > 0)
                {
                    fixed (byte** inputsToRust = &inputPtrs[0])
                    {
                        exc = InitExpression(expressionPtr, inputsToRust, (nuint)rawInputs.Length);
                    }
                    for (int i = 0; i < pinnedInputs.Length; i++)
                    {
                        pinnedInputs[i].Free();
                    }
                    pinnedExpression.Free();
                }
                else
                {
                    exc = InitExpression(expressionPtr, null, 0);
                }

                if (exc != null) throw exc;
            }
        }

        private unsafe KuiperException ExceptionFromError(KuiperError error)
        {
            if (!error.is_error)
            {
                return null;
            }

            string msg = "";
            if (((IntPtr)error.error) != IntPtr.Zero)
            {
                msg = PointerToStringUTF8(error.error);
            }
            return new KuiperException(msg, error.start, error.end);
        }

        private unsafe KuiperException InitExpression(byte* expressionPtr, byte** inputsToRust, nuint inputsLength)
        {
            KuiperException exc = null;
            var result = KuiperInterop.compile_expression(expressionPtr, inputsToRust, inputsLength);
            exc = ExceptionFromError((*result).error);
            if (exc == null)
            {
                _expression = KuiperInterop.get_expression_from_compile_result(result);
            }

            return exc;
        }

        private unsafe string PointerToStringUTF8(byte* input)
        {
            int length = 0;
            for (byte* i = input; i[length] != 0; length++) ;

            return Encoding.UTF8.GetString(input, length);
        }

        /// <summary>
        /// Run a Kuiper expression.
        /// </summary>
        /// <param name="inputs">JSON strings passed as arguments, the number must be equal
        /// to the `inputs` array passed to the constructor.</param>
        /// <returns>JSON string result.</returns>
        public string Run(params string[] inputs)
        {
            unsafe
            {
                var rawInputs = inputs.Select(inp => Encoding.UTF8.GetBytes(inp + char.MinValue)).ToArray();

                GCHandle[] pinnedInputs = new GCHandle[rawInputs.Length];
                byte*[] inputPtrs = new byte*[rawInputs.Length];

                for (int i = 0; i < rawInputs.Length; i++)
                {
                    pinnedInputs[i] = GCHandle.Alloc(rawInputs[i], GCHandleType.Pinned);
                    inputPtrs[i] = (byte*)pinnedInputs[i].AddrOfPinnedObject();
                }

                KuiperException exc = null;
                string transformedData = null;
                if (inputPtrs.Length > 0)
                {
                    fixed (byte** inputsToRust = &inputPtrs[0])
                    {
                        exc = RunInternal(inputsToRust, (nuint)rawInputs.Length, out transformedData);
                        for (int i = 0; i < pinnedInputs.Length; i++)
                        {
                            pinnedInputs[i].Free();
                        }
                    }
                }
                else
                {
                    exc = RunInternal(null, 0, out transformedData);
                }

                if (exc != null) throw exc;
                return transformedData;
            }
        }

        private unsafe KuiperException RunInternal(byte** inputsToRust, nuint inputsLength, out string transformedData)
        {
            KuiperException exc = null;
            transformedData = null;
            var result = KuiperInterop.run_expression(inputsToRust, inputsLength, _expression);
            exc = ExceptionFromError((*result).error);
            if (exc == null)
            {
                transformedData = PointerToStringUTF8((*result).result);
                KuiperInterop.destroy_transform_result(result);
            }
            return exc;
        }

        /// <inheritdoc />
        public override string ToString()
        {
            unsafe
            {
                var result = KuiperInterop.expression_to_string(_expression);
                string resString = PointerToStringUTF8(result);
                KuiperInterop.destroy_string(result);
                return resString;
            }
        }

        ~KuiperExpression() => Dispose(false);

        private bool disposedValue;

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (_expression != IntPtr.Zero)
                {
                    KuiperInterop.destroy_expression(_expression);
                    _expression = IntPtr.Zero;
                }

                disposedValue = true;
            }
        }

        /// <inheritdoc />
        public void Dispose()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }
    }
}
