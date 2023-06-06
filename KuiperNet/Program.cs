using System;

namespace Cognite.Kuiper {
    public static class Program {
        static void Main() {
            using var expression = new Kuiper("input.value + 4", new [] { "input" });
            // var result = expression.Execute("{ \"value\": 5 }");
            // Console.WriteLine($"Result: {result}");
        }
    }
}
