using FluentAssertions;
using TechJobsNL.Core.Application.Progress;

namespace TechJobsNL.Core.Tests.Application.Progress;

public sealed class ProgressStreamsTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-004")]
    [Trait("Category", "Unit")]
    public async Task PublishAsync_CapacityOne_WaitsThenDeliversMessagesInOrder()
    {
        var stream = ProgressStreams.Create<string>(1);

        await stream.PublishAsync("first", TestContext.Current.CancellationToken);
        var secondWrite = stream.PublishAsync("second", TestContext.Current.CancellationToken);

        secondWrite.IsCompleted.Should().BeFalse();
        (await stream.Reader.ReadAsync(TestContext.Current.CancellationToken)).Should().Be("first");
        await secondWrite;
        (await stream.Reader.ReadAsync(TestContext.Current.CancellationToken)).Should().Be("second");
        stream.Complete();

        (await stream.Reader.WaitToReadAsync(TestContext.Current.CancellationToken)).Should().BeFalse();
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-004")]
    [Trait("Category", "Unit")]
    public async Task PublishAsync_BlockedWriter_PropagatesCancellation()
    {
        var stream = ProgressStreams.Create<string>(1);
        using var cancellation = new CancellationTokenSource();
        await stream.PublishAsync("first", TestContext.Current.CancellationToken);
        var blockedWrite = stream.PublishAsync("second", cancellation.Token);
        cancellation.Cancel();

        var action = async () => await blockedWrite.ConfigureAwait(false);

        await action.Should().ThrowAsync<OperationCanceledException>().ConfigureAwait(true);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-004")]
    [Trait("Category", "Unit")]
    public void Create_NonPositiveCapacity_RejectsTheInvalidContract()
    {
        var action = () => ProgressStreams.Create<string>(0);

        action.Should().Throw<ArgumentOutOfRangeException>();
    }
}
