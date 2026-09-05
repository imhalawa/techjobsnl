using System.Threading.Channels;

namespace TechJobsNL.Core.Application.Progress;

/// <summary>Creates bounded FIFO progress streams.</summary>
public static class ProgressStreams
{
    /// <summary>Creates a stream with positive capacity and wait-on-full backpressure.</summary>
    public static IProgressStream<TProgress> Create<TProgress>(int capacity)
    {
        if (capacity <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(capacity), capacity, "Progress capacity must be positive.");
        }

        var options = new BoundedChannelOptions(capacity)
        {
            FullMode = BoundedChannelFullMode.Wait,
            SingleReader = true,
            SingleWriter = true
        };
        return new BoundedProgressStream<TProgress>(Channel.CreateBounded<TProgress>(options));
    }

    private sealed class BoundedProgressStream<TProgress> : IProgressStream<TProgress>
    {
        private readonly Channel<TProgress> _channel;

        public BoundedProgressStream(Channel<TProgress> channel)
        {
            _channel = channel;
        }

        public ChannelReader<TProgress> Reader => _channel.Reader;

        public Task PublishAsync(TProgress progress, CancellationToken cancellationToken) =>
            _channel.Writer.WriteAsync(progress, cancellationToken).AsTask();

        public void Complete(Exception? error = null) => _channel.Writer.TryComplete(error);
    }
}
