unit TestCore;

interface

uses
  TestFramework;

type
  TCoreTests = class(TTestCase)
  published
    procedure ExamplePasses;
  end;

implementation

procedure TCoreTests.ExamplePasses;
begin
  CheckTrue(True);
end;

initialization
  RegisterTest(TCoreTests.Suite);

end.

