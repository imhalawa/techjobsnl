using System.Text.RegularExpressions;

namespace TechJobsNL.Core.Operations;

/// <summary>Removes credentials and embedded response bodies from displayable operational diagnostics.</summary>
public static partial class OperationalRedaction
{
    /// <summary>Retains diagnostic context while masking sensitive URL components, credentials, and payloads.</summary>
    public static string? Redact(string? diagnostic)
    {
        if (diagnostic is null)
        {
            return null;
        }

        var result = Payload().Replace(diagnostic, "[redacted payload]");
        result = Url().Replace(result, static match =>
        {
            if (!Uri.TryCreate(match.Value, UriKind.Absolute, out var uri))
            {
                return "[redacted URL]";
            }

            var safe = uri.GetComponents(UriComponents.SchemeAndServer | UriComponents.Path, UriFormat.UriEscaped);
            return uri.Query.Length != 0 || uri.Fragment.Length != 0 ? safe + "?[redacted]" : safe;
        });
        result = Authorization().Replace(result, "${name}=[redacted]");
        result = Bearer().Replace(result, "Bearer [redacted]");
        return SecretAssignment().Replace(result, "${name}=[redacted]");
    }

    [GeneratedRegex("""\b(?<name>(?:access[_-]?|refresh[_-]?)?token|api[_-]?key|secret|password)\s*[=:]\s*(?:"[^"]*"|'[^']*'|[^\s;&,]+)""", RegexOptions.IgnoreCase | RegexOptions.ExplicitCapture, 1000)]
    private static partial Regex SecretAssignment();

    [GeneratedRegex(@"\b(?<name>(?:proxy-)?authorization|(?:set-)?cookie)\s*[=:]\s*[^\r\n]+", RegexOptions.IgnoreCase | RegexOptions.ExplicitCapture, 1000)]
    private static partial Regex Authorization();

    [GeneratedRegex(@"\bBearer\s+[^\s;,]+", RegexOptions.IgnoreCase | RegexOptions.ExplicitCapture, 1000)]
    private static partial Regex Bearer();

    [GeneratedRegex(@"https?://[^\s]+", RegexOptions.IgnoreCase | RegexOptions.ExplicitCapture, 1000)]
    private static partial Regex Url();

    [GeneratedRegex("""(?:\{|<(?=[!?/A-Za-z])|\[\s*(?=["\{\[\d\-]|true\b|false\b|null\b)).*""", RegexOptions.Singleline | RegexOptions.ExplicitCapture, 1000)]
    private static partial Regex Payload();
}
