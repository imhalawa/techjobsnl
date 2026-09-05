namespace TechJobsNL.Core.Domain.Configuration;

/// <summary>Defines retained scanning settings.</summary>
public sealed record ScanConfiguration
{
    /// <summary>Initializes a new instance of the <see cref="ScanConfiguration"/> class.</summary>
    public ScanConfiguration(int concurrency, long timeoutSeconds, int retryCount, string userAgent)
    {
        Concurrency = concurrency;
        TimeoutSeconds = timeoutSeconds;
        RetryCount = retryCount;
        UserAgent = userAgent;
    }

    /// <summary>Gets the maximum concurrent company scans.</summary>
    public int Concurrency { get; }

    /// <summary>Gets the retained per-source timeout.</summary>
    public long TimeoutSeconds { get; }

    /// <summary>Gets the retry budget.</summary>
    public int RetryCount { get; }

    /// <summary>Gets the retained HTTP user agent.</summary>
    public string UserAgent { get; }
}
