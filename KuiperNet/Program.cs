using System;

namespace Cognite.Kuiper
{
    public static class Program
    {
        static void Main()
        {
            using var expression = new KuiperExpression(@"[0, 1, 2, 3].map(a => a + input.test)", new[] { "input" });
            var result = expression.Run(new[] { @"{ ""test"": 2 }" });
            Console.WriteLine(result);
        }
    }
}

