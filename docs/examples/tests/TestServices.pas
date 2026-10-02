unit TestServices;

interface

uses
  TestFramework;

type
  TServiceTests = class(TTestCase)
  published
    procedure ExamplePasses;
  end;

implementation

procedure TServiceTests.ExamplePasses;
begin
  CheckEquals('service', 'service');
end;

initialization
  RegisterTest(TServiceTests.Suite);

end.

