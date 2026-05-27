@echo off
echo === Firecrawl Root ===
curl.exe -s --max-time 5 http://localhost:3002/
echo.

echo === Firecrawl v1/search (query=test, limit=2) ===
curl.exe -s --max-time 15 -X POST http://localhost:3002/v1/search -H "Content-Type: application/json" -d "{\"query\":\"test\",\"limit\":2}"
echo.
echo EXIT CODE: %ERRORLEVEL%

echo.
echo === Firecrawl v1/scrape (example.com) ===
curl.exe -s --max-time 15 -X POST http://localhost:3002/v1/scrape -H "Content-Type: application/json" -d "{\"url\":\"https://example.com\"}"
echo.
echo EXIT CODE: %ERRORLEVEL%
