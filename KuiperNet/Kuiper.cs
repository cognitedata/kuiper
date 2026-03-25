using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.ExceptionServices;
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
        public unsafe RawKuiperExpression* result;
#pragma warning restore CS0649
    }

    internal struct TransformResult
    {
#pragma warning disable CS0649 // These fields are assigned in external code.
        public KuiperError error;
        public unsafe byte* result;
#pragma warning restore CS0649
    }

    internal struct RawKuiperExpression { }

    internal struct RawCompilerConfig { }

    internal struct CustomFunctionResult
    {
        public bool isError;
        public unsafe byte* data;
        public IntPtr free_payload;
        public IntPtr freeData;
    }

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    internal unsafe delegate void FreeDataCallback(byte* data);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    internal unsafe delegate CustomFunctionResult CustomFunctionCallback(byte** args, UIntPtr argsLen);

    internal static class KuiperInterop
    {
        public const string NativeLib = "kuiper_interop";

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "compile_expression")]
        public unsafe static extern CompileResult* compile_expression(byte* data, byte** inputs, UIntPtr len);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "destroy_compile_result")]
        public unsafe static extern void destroy_compile_result(CompileResult* data);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "get_expression_from_compile_result")]
        public unsafe static extern RawKuiperExpression* get_expression_from_compile_result(CompileResult* result);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "destroy_transform_result")]
        public unsafe static extern void destroy_transform_result(TransformResult* result);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "run_expression")]
        public unsafe static extern TransformResult* run_expression(byte** data, UIntPtr len, RawKuiperExpression* expression);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "destroy_expression")]
        public unsafe static extern void destroy_expression(RawKuiperExpression* data);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "expression_to_string")]
        public unsafe static extern byte* expression_to_string(RawKuiperExpression* expression);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "destroy_string")]
        public unsafe static extern void destroy_string(byte* data);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "new_compiler_config")]
        public unsafe static extern RawCompilerConfig* new_compiler_config();

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "destroy_compiler_config")]
        public unsafe static extern void destroy_compiler_config(RawCompilerConfig* config);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "config_set_optimizer_operation_limit")]
        public unsafe static extern void config_set_optimizer_operation_limit(RawCompilerConfig* config, long limit);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "config_set_max_macro_expansions")]
        public unsafe static extern void config_set_max_macro_expansions(RawCompilerConfig* config, int limit);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "config_add_custom_function")]
        public unsafe static extern int config_add_custom_function(RawCompilerConfig* config, byte* name, IntPtr callback);

        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "compile_expression_with_config")]
        public unsafe static extern CompileResult* compile_expression_with_config(byte* expression, byte** inputs, UIntPtr inputs_len, RawCompilerConfig* config);
    }

    internal static class Utils
    {
        public static unsafe string PointerToStringUTF8(byte* input)
        {
            int length = 0;
            for (byte* i = input; i[length] != 0; length++) ;

            return Encoding.UTF8.GetString(input, length);
        }
    }

    public sealed class KuiperExpression : IDisposable
    {
        private unsafe RawKuiperExpression* _expression;
        private static CompilerConfig defaultConfig = new CompilerConfig();

        private readonly CompilerConfig _config;

        /// <summary>
        /// Compile a kuiper expression.
        ///
        /// This will throw a `KuiperException` if compilation failed.
        /// </summary>
        /// <param name="expression">Expression code</param>
        /// <param name="inputs">A list of available input arguments</param>
        public KuiperExpression(string expression, params string[] inputs) : this(expression, defaultConfig, inputs)
        {
        }

        public KuiperExpression(string expression, CompilerConfig config, params string[] inputs)
        {
            _config = config;
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
                        exc = InitExpression(expressionPtr, inputsToRust, (nuint)rawInputs.Length, config.GetRawConfig());
                    }
                    for (int i = 0; i < pinnedInputs.Length; i++)
                    {
                        pinnedInputs[i].Free();
                    }
                    pinnedExpression.Free();
                }
                else
                {
                    exc = InitExpression(expressionPtr, null, 0, config.GetRawConfig());
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
                msg = Utils.PointerToStringUTF8(error.error);
            }
            return new KuiperException(msg, error.start, error.end);
        }

        private unsafe KuiperException InitExpression(byte* expressionPtr, byte** inputsToRust, nuint inputsLength, RawCompilerConfig* config)
        {
            KuiperException exc = null;
            var result = KuiperInterop.compile_expression_with_config(expressionPtr, inputsToRust, inputsLength, config);
            exc = ExceptionFromError((*result).error);
            if (exc == null)
            {
                _expression = KuiperInterop.get_expression_from_compile_result(result);
            }

            return exc;
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
                if (exc != null) ExceptionDispatchInfo.Capture(exc).Throw();
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
                transformedData = Utils.PointerToStringUTF8((*result).result);
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
                string resString = Utils.PointerToStringUTF8(result);
                KuiperInterop.destroy_string(result);
                return resString;
            }
        }

        ~KuiperExpression() => Dispose(false);

        private bool disposedValue;

        private void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                unsafe
                {
                    if (_expression != null)
                    {
                        KuiperInterop.destroy_expression(_expression);
                        _expression = null;
                    }
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

    /// <summary>
    /// Optional configuration for kuiper expressions.
    /// </summary>
    public sealed class CompilerConfig : IDisposable
    {
        private unsafe RawCompilerConfig* _config;

        /// <summary>
        /// Note: This appears unused, but _must_ be kept alive since the custom functions
        /// may be called from native code at any time.
        /// </summary>
        private readonly List<CustomFunctionCallback> _customFunctionCallbacks = new List<CustomFunctionCallback>();

        /// <summary>
        /// Construct a new empty compiler config.
        /// </summary>
        public CompilerConfig()
        {
            unsafe
            {
                _config = KuiperInterop.new_compiler_config();
            }
        }

        /// <summary>
        /// Set a limit on the number of operations the optimizer will perform when optimizing an expression.
        /// </summary>
        /// <param name="limit">The maximum number of operations.</param>
        public CompilerConfig SetOptimizerOperationLimit(long limit)
        {
            unsafe
            {
                KuiperInterop.config_set_optimizer_operation_limit(_config, limit);
            }
            return this;
        }


        /// <summary>
        /// Set a limit on the number of macro expansions that will be performed when compiling an expression.
        /// </summary>
        /// <param name="limit">Maximum number of macro expansions.</param>
        public CompilerConfig SetMaxMacroExpansions(int limit)
        {
            unsafe
            {
                KuiperInterop.config_set_max_macro_expansions(_config, limit);
            }
            return this;
        }

        unsafe static FreeDataCallback freeDataDelegate = new FreeDataCallback(data =>
        {
            var handle = GCHandle.FromIntPtr((IntPtr)data);
            handle.Free();
        });

        /// <summary>
        /// Add a custom function that can be called from kuiper expressions.
        /// </summary>
        /// <param name="name">The name of the custom function.</param>
        /// <param name="callback">The callback function to be invoked.</param>
        /// <exception cref="InvalidOperationException">If adding the custom function fails for some reason.</exception>
        public CompilerConfig AddCustomFunction(string name, Func<string[], string> callback)
        {
            unsafe
            {
                var rawName = Encoding.UTF8.GetBytes(name + char.MinValue);
                var pinnedName = GCHandle.Alloc(rawName, GCHandleType.Pinned);
                var namePtr = (byte*)pinnedName.AddrOfPinnedObject();

                CustomFunctionResult inner(byte** args, UIntPtr argsLen)
                {
                    var strings = new string[argsLen.ToUInt64()];
                    for (ulong i = 0; i < argsLen.ToUInt64(); i++)
                    {
                        strings[i] = Utils.PointerToStringUTF8(args[i]);
                    }
                    string result;
                    bool isError = false;
                    try
                    {
                        result = callback(strings);
                    }
                    catch (Exception ex)
                    {
                        isError = true;
                        result = ex.Message;
                    }

                    var rawResult = Encoding.UTF8.GetBytes(result + char.MinValue);
                    var pinnedResult = GCHandle.Alloc(rawResult, GCHandleType.Pinned);
                    var resultPtr = GCHandle.ToIntPtr(pinnedResult);

                    return new CustomFunctionResult
                    {
                        isError = isError,
                        data = (byte*)pinnedResult.AddrOfPinnedObject(),
                        free_payload = resultPtr,
                        freeData = Marshal.GetFunctionPointerForDelegate(freeDataDelegate)
                    };
                }

                var dg = new CustomFunctionCallback(inner);
                _customFunctionCallbacks.Add(dg);

                IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(dg);

                int res = KuiperInterop.config_add_custom_function(_config, namePtr, callbackPtr);
                pinnedName.Free();

                if (res != 0)
                {
                    throw new InvalidOperationException($"Failed to add custom function {name} to compiler config");
                }
            }
            return this;
        }

        internal unsafe RawCompilerConfig* GetRawConfig() => _config;

        ~CompilerConfig() => Dispose(false);

        private bool disposedValue;

        private void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                unsafe
                {
                    if (_config != null)
                    {
                        KuiperInterop.destroy_compiler_config(_config);
                        _config = null;
                    }
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
