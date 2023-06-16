using System;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;

namespace Cognite.Kuiper
{
    public class KuiperException : Exception
    {
        public KuiperException(string message) : base(message) { }
    }

    internal struct CompileResult
    {
        public unsafe byte* error;
        public IntPtr result;
    }

    internal struct TransformResult
    {
        public unsafe byte* error;
        public unsafe byte* result;
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
    }

    public class KuiperExpression : IDisposable
    {
        private IntPtr _expression;

        public KuiperExpression(string expression, string[] inputs)
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
                fixed (byte** inputsToRust = &inputPtrs[0])
                {
                    var result = KuiperInterop.compile_expression(expressionPtr, inputsToRust, (nuint)rawInputs.Length);
                    if (((IntPtr)(*result).error) != IntPtr.Zero)
                    {
                        string error = Marshal.PtrToStringUTF8((nint)(*result).error);
                        exc = new KuiperException(error);
                        KuiperInterop.destroy_compile_result(result);
                    }
                    else
                    {
                        _expression = KuiperInterop.get_expression_from_compile_result(result);
                    }

                    for (int i = 0; i < pinnedInputs.Length; i++)
                    {
                        pinnedInputs[i].Free();
                    }
                    pinnedExpression.Free();
                }
                if (exc != null) throw exc;
            }
        }

        public string Run(string[] inputs)
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
                fixed (byte** inputsToRust = &inputPtrs[0])
                {
                    var result = KuiperInterop.run_expression(inputsToRust, (nuint)rawInputs.Length, _expression);
                    if (((IntPtr)(*result).error) != IntPtr.Zero)
                    {
                        string error = Marshal.PtrToStringUTF8((nint)(*result).error);
                        exc = new KuiperException(error);
                        KuiperInterop.destroy_transform_result(result);
                    }
                    else
                    {
                        transformedData = Marshal.PtrToStringUTF8((nint)(*result).result);
                        KuiperInterop.destroy_transform_result(result);
                    }
                    for (int i = 0; i < pinnedInputs.Length; i++)
                    {
                        pinnedInputs[i].Free();
                    }
                }
                if (exc != null) throw exc;
                return transformedData;
            }
        }

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

        public void Dispose()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }
    }
}