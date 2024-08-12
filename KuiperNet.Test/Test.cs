using Xunit;
using Cognite.Kuiper;

namespace KuiperNet.Test;


public class UnitTest1
{
    [Fact]
    public void TestKuiperExpressionNoArgs()
    {
        var expr = new KuiperExpression("1 + 1", []);
        Assert.Equal("2", expr.Run());
        Assert.Equal("2", expr.ToString());
    }

    [Fact]
    public void TestKuiperCompileErr()
    {
        var ex = Assert.Throws<KuiperException>(() => new KuiperExpression("\"test\".notafunc()", []));
        Assert.Equal("Compilation failed: Unrecognized function: notafunc at 7..17", ex.Message);
        Assert.Equal(7ul, ex.Start);
        Assert.Equal(17ul, ex.End);
    }

    [Fact]
    public void TestKuiperWithInputs()
    {
        var expr = new KuiperExpression("in1 + in2 + in3", ["in1", "in2", "in3"]);
        Assert.Equal("6", expr.Run("1", "2", "3"));
    }
}
