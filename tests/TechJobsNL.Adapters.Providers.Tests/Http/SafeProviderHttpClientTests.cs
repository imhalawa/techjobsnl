using System.Net;
using FluentAssertions;
using TechJobsNL.Adapters.Providers.Http;
using TechJobsNL.Core.Domain;

namespace TechJobsNL.Adapters.Providers.Tests.Http;

public sealed class SafeProviderHttpClientTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-009")]
    public async Task GetTextAsync_Success_ReturnsBodyAndCanonicalUrl()
    {
        using var handler = new ScriptedHandler(_ => Response(HttpStatusCode.OK, "official", "https://jobs.example.test/list/"));
        using var client = new HttpClient(handler);
        var subject = new SafeProviderHttpClient(client, ["jobs.example.test"]);

        var result = await subject.GetTextAsync(new Uri("https://jobs.example.test/list/"), "source", TestContext.Current.CancellationToken);

        result.Should().Be(new ProviderHttpResult.Success("official", new Uri("https://jobs.example.test/list")));
        handler.RequestCount.Should().Be(1);
    }

    [Theory]
    [Trait("TaskId", "V0.1.0-009")]
    [InlineData(408, SourceErrorKind.Timeout, true)]
    [InlineData(429, SourceErrorKind.RateLimit, true)]
    [InlineData(500, SourceErrorKind.Transport, true)]
    [InlineData(401, SourceErrorKind.Transport, false)]
    [InlineData(403, SourceErrorKind.Transport, false)]
    public async Task GetTextAsync_Status_MapsRustFailure(int status, SourceErrorKind kind, bool retryable)
    {
        using var handler = new ScriptedHandler(_ => Response((HttpStatusCode)status, "denied", "https://jobs.example.test"));
        using var client = new HttpClient(handler);
        var subject = new SafeProviderHttpClient(client, ["jobs.example.test"]);

        var result = await subject.GetTextAsync(new Uri("https://jobs.example.test"), "Ashby", TestContext.Current.CancellationToken);

        var failure = result.Should().BeOfType<ProviderHttpResult.Failure>().Which.Error;
        failure.Should().Match<ProviderFailure>(value => value.Kind == kind && value.HttpStatus == status && value.IsRetryable == retryable);
        ProviderRetry.ShouldRetry(result).Should().Be(retryable);
        handler.RequestCount.Should().Be(1);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-009")]
    public async Task GetTextAsync_RateLimit_ParsesRetryAfterSeconds()
    {
        using var handler = new ScriptedHandler(_ => { var response = Response(HttpStatusCode.TooManyRequests, "", "https://jobs.example.test"); response.Headers.TryAddWithoutValidation("Retry-After", "9"); return response; });
        using var client = new HttpClient(handler);

        var result = await new SafeProviderHttpClient(client, ["jobs.example.test"]).GetTextAsync(new Uri("https://jobs.example.test"), "Ashby", TestContext.Current.CancellationToken);

        result.Should().BeOfType<ProviderHttpResult.Failure>().Which.Error.RetryAfter.Should().Be(TimeSpan.FromSeconds(9));
    }

    [Theory]
    [Trait("TaskId", "V0.1.0-009")]
    [InlineData("http://jobs.example.test")]
    [InlineData("https://attacker.example.test")]
    public async Task GetTextAsync_UnsafeInitialUri_IsBlockedWithoutNetwork(string uri)
    {
        using var handler = new ScriptedHandler(_ => Response(HttpStatusCode.OK, "", uri));
        using var client = new HttpClient(handler);

        var result = await new SafeProviderHttpClient(client, ["jobs.example.test"]).GetTextAsync(new Uri(uri), "source", TestContext.Current.CancellationToken);

        result.Should().BeOfType<ProviderHttpResult.Failure>().Which.Error.IsRetryable.Should().BeFalse();
        handler.RequestCount.Should().Be(0);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-009")]
    public async Task GetTextAsync_UnsafeRedirectTarget_IsBlocked()
    {
        using var handler = new ScriptedHandler(_ => Response(HttpStatusCode.OK, "stolen", "https://attacker.example.test"));
        using var client = new HttpClient(handler);

        var result = await new SafeProviderHttpClient(client, ["jobs.example.test"]).GetTextAsync(new Uri("https://jobs.example.test"), "source", TestContext.Current.CancellationToken);

        result.Should().BeOfType<ProviderHttpResult.Failure>().Which.Error.Kind.Should().Be(SourceErrorKind.Configuration);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-009")]
    public async Task RetryPipeline_NonRetryableFailure_IsNotRetried()
    {
        var attempts = 0;
        var failure = new ProviderHttpResult.Failure(new ProviderFailure(SourceErrorKind.Transport, "denied", 403, null, false));

        var result = await ProviderRetry.CreatePipeline(3, TimeSpan.Zero).ExecuteAsync(_ => { attempts++; return ValueTask.FromResult<ProviderHttpResult>(failure); }, TestContext.Current.CancellationToken);

        result.Should().BeSameAs(failure);
        attempts.Should().Be(1);
    }

    private static HttpResponseMessage Response(HttpStatusCode status, string body, string finalUri) => new(status) { Content = new StringContent(body), RequestMessage = new HttpRequestMessage(HttpMethod.Get, finalUri) };

    private sealed class ScriptedHandler(Func<HttpRequestMessage, HttpResponseMessage> response) : HttpMessageHandler
    {
        public int RequestCount { get; private set; }
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken) { cancellationToken.ThrowIfCancellationRequested(); RequestCount++; return Task.FromResult(response(request)); }
    }
}
