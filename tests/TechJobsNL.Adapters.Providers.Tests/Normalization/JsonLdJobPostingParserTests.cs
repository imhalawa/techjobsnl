using FluentAssertions;
using TechJobsNL.Adapters.Providers.Normalization;
using TechJobsNL.Core.Domain;

namespace TechJobsNL.Adapters.Providers.Tests.Normalization;

public sealed class JsonLdJobPostingParserTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-009")]
    public void Parse_GraphJobPosting_ReturnsTypedDataAndNormalizesHtml()
    {
        const string html = """
            <script type="application/ld+json">{"@graph":[{"@type":"BreadcrumbList"},{"@type":"JobPosting","identifier":"REQ-42","url":"https://example.test/job","title":"Platform Engineer","description":"<p>Build &amp; ship <strong>reliable</strong> systems.</p>","employmentType":["FULL_TIME"],"jobLocation":{"address":{"addressCountry":"NL"}},"hiringOrganization":{"name":"Example"}}]}</script>
            """;

        var posting = JsonLdJobPostingParser.Parse(html, "fixture").Should().BeOfType<ProviderNormalizationResult<JobPosting>.Success>().Which.Value;

        posting.Title.Should().Be("Platform Engineer");
        JsonLdJobPostingParser.HtmlText(posting.Description).Should().Be("Build & ship reliable systems.");
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-009")]
    public void Parse_MalformedJsonLdBeforeValidPosting_ReturnsNonRetryableSchemaFailure()
    {
        const string html = """
            <script type="application/ld+json">{"@type":"BreadcrumbList"</script>
            <script type="application/ld+json">{"@type":"JobPosting","identifier":"one","description":"valid"}</script>
            """;

        var failure = JsonLdJobPostingParser.Parse(html, "mixed fixture").Should().BeOfType<ProviderNormalizationResult<JobPosting>.Failure>().Which.Error;

        failure.Kind.Should().Be(SourceErrorKind.Schema);
        failure.IsRetryable.Should().BeFalse();
        failure.Message.Should().Contain("invalid JSON-LD");
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-009")]
    public void Parse_MissingPosting_ReturnsSchemaFailure()
    {
        JsonLdJobPostingParser.Parse("<html></html>", "empty").Should().BeOfType<ProviderNormalizationResult<JobPosting>.Failure>()
            .Which.Error.Kind.Should().Be(SourceErrorKind.Schema);
    }
}
