using System.Threading.Channels;

namespace TechJobsNL.Core.Application.Progress;

/// <summary>Provides ordered, bounded progress messages for one operation lifetime.</summary>
/// <typeparam name="TProgress">An immutable progress message type.</typeparam>
public interface IProgressStream<TProgress>
{
    /// <summary>Gets the ordered reader for the operation's presentation consumer.</summary>
    ChannelReader<TProgress> Reader { get; }

    /// <summary>Publishes one message, waiting for bounded capacity when necessary.</summary>
    Task PublishAsync(TProgress progress, CancellationToken cancellationToken);

    /// <summary>Completes the stream normally or with the owning operation's unhandled failure.</summary>
    void Complete(Exception? error = null);
}
