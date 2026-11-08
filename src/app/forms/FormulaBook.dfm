object FormulaBookFrm: TFormulaBookFrm
  Left = 200
  Top = 150
  Caption = '@MB:FormulaBook'
  ClientHeight = 580
  ClientWidth = 900
  Color = clBtnFace
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  OldCreateOrder = False
  Position = poScreenCenter
  OnShow = FormShow
  PixelsPerInch = 96
  TextHeight = 13
  object Label1: TLabel
    Left = 8
    Top = 12
    Width = 40
    Height = 13
    Caption = '@MB:Search'
  end
  object SearchEdit: TEdit
    Left = 56
    Top = 8
    Width = 200
    Height = 21
    TabOrder = 0
    OnChange = FilterChange
  end
  object CatCombo: TComboBox
    Left = 8
    Top = 36
    Width = 248
    Height = 21
    Style = csDropDownList
    ItemHeight = 13
    TabOrder = 1
    OnChange = FilterChange
  end
  object NameList: TListBox
    Left = 8
    Top = 64
    Width = 248
    Height = 476
    ItemHeight = 13
    TabOrder = 2
    OnClick = NameListClick
    OnDblClick = NameListDblClick
  end
  object CountLabel: TLabel
    Left = 8
    Top = 548
    Width = 248
    Height = 13
    Caption = ''
  end
  object Image1: TImage
    Left = 268
    Top = 8
    Width = 320
    Height = 240
    Center = True
  end
  object PicLabel: TLabel
    Left = 268
    Top = 252
    Width = 320
    Height = 28
    AutoSize = False
    WordWrap = True
    Caption = ''
  end
  object NameLabel: TLabel
    Left = 600
    Top = 8
    Width = 292
    Height = 16
    AutoSize = False
    Caption = ''
    Font.Charset = DEFAULT_CHARSET
    Font.Color = clWindowText
    Font.Height = -13
    Font.Name = 'MS Sans Serif'
    Font.Style = [fsBold]
    ParentFont = False
  end
  object KindLabel: TLabel
    Left = 600
    Top = 28
    Width = 292
    Height = 16
    AutoSize = False
    Caption = ''
  end
  object Label5: TLabel
    Left = 600
    Top = 48
    Width = 292
    Height = 13
    Caption = '@MB:ExampleFiles'
  end
  object ExampleList: TListBox
    Left = 600
    Top = 64
    Width = 292
    Height = 216
    ItemHeight = 13
    TabOrder = 3
    OnDblClick = ExampleListDblClick
  end
  object PageMemo: TMemo
    Left = 268
    Top = 288
    Width = 624
    Height = 252
    ReadOnly = True
    ScrollBars = ssVertical
    TabOrder = 4
  end
  object UseBtn: TButton
    Left = 268
    Top = 548
    Width = 200
    Height = 25
    Caption = '@MB:UseInTheFormulasWindow'
    TabOrder = 5
    OnClick = UseBtnClick
  end
  object OpenBtn: TButton
    Left = 476
    Top = 548
    Width = 160
    Height = 25
    Caption = '@MB:OpenTheExample'
    TabOrder = 6
    OnClick = OpenBtnClick
  end
  object CopyBtn: TButton
    Left = 644
    Top = 548
    Width = 140
    Height = 25
    Caption = '@MB:CopyTheName'
    TabOrder = 7
    OnClick = CopyBtnClick
  end
  object CloseBtn: TButton
    Left = 800
    Top = 548
    Width = 92
    Height = 25
    Caption = '@MB:Close'
    TabOrder = 8
    OnClick = CloseBtnClick
  end
end
