import SwiftUI

// Made by the label import (the desktop's Languages/English.lng): the
// Formula book's labels with their English texts, which are the keys of
// the app's translations.
enum BookLabels {
    static let english: [String: String] = [
        "N3DEscapeTimeFormulas": "3D escape-time formulas",
        "N3DFormulasWithAnalyticDEFast": "3D formulas with analytic DE (fast)",
        "N4DFormulas": "4D formulas",
        "N4DFormulasWithAnalyticDE": "4D formulas with analytic DE",
        "TransformationsOnlyInCombination": "Transformations (only in combination)",
        "DIFSShapes": "dIFS shapes",
        "DIFSTransformations": "dIFS transformations",
        "A3DEscapeTimeFormulaWith": "A 3D escape-time formula with a numerical distance estimate: the shape works alone and combines with others; slower than analytic DE.",
        "A3DEscapeTimeFormulaWith2": "A 3D escape-time formula with an analytic distance estimate: fast and robust; a good first formula.",
        "A4DFormulaTheFourthCoordinate": "A 4D formula: the fourth coordinate and the 4D rotation of the Formulas window apply; numerical DE, so it is slow.",
        "A4DFormulaWithAnAnalytic": "A 4D formula with an analytic distance estimate: the 4D rotation applies, and it renders fast.",
        "ATransformationItChangesThePoint": "A transformation: it changes the point only and has no shape of its own. Use it in a slot before or between shaping formulas, never alone.",
        "ADIFSShapeADistanceEstimated": "A dIFS shape: a distance-estimated solid (sphere, box, tube ...). Combine it only with other dIFS formulas; mixing it with escape-time formulas gives the error 'formula option is not valid'.",
        "ADIFSTransformationItBendsRepeats": "A dIFS transformation: it bends, repeats or cuts the dIFS shapes in the slots after it.",
        "TheTriplexMathSineBulbAs": "The triplex math sine bulb, as suggested by Daniel White and Paul Nylander.  These are fast implementions of powers of 2,3,4,5,6,7 and 8.\n\nIf you want to make an animation with smooth power changings, please use the 'Real Power' formula.",
        "TheSineBulbWhereArbitraryPower": "The sine bulb where arbitrary power values can be used, the calculation is slower than of integer powers.\n\nr  = sqrt(x*x + y*y + z*z)\nth = ArcTan2(y, x) * Float_power\nph = ArcSin(z/r) * Float_power\nr  = Power(r, Float_power)\nx  = r * cos(ph) * cos(th) + Cx\ny  = r * cos(ph) * sin(th) + Cy\nz  = Z_multiplier * r * sin(ph) + Cz",
        "A4dQuaternionFormulaOneIteration": "A 4d quaternion formula, one iteration:\n\nx' = x*x - y*y - z*z - w*w + Cx\ny' = 2*(y*x + z*w) + Cy\nz' = 2*(z*x + YW_multiplier*y*w) + Cz\nw' = 2*(w*x + y*z) + W_add + Cw",
        "TheAmazingBoxAkaMandboxFormula": "The 'Amazing Box' aka Mandbox formula, invented by TGlad at fractalforums.com.\n\nThe formula for one iteration looks like this:\n\nx = abs(x+Fold) - abs(x-Fold) - x\ny = abs(y+Fold) - abs(y-Fold) - y\nz = abs(z+Fold) - abs(z-Fold) - z\nrr = x*x + y*y + z*z\nif rr < sqr(Min_R) then m = Scale/sqr(Min_R) else\nif rr < 1 then m = Scale/rr else m = Scale\nx = x * m + Cx\ny = y * m + Cy\nz = z * m + Cz",
        "AHybridOfASinePower": "A hybrid of a sine power 2 bulb and the amazing box, the 3d vector length decides which formula is used:\nIf it is bigger than 'Box/Bulb R threshold' the amazing box is used, else if smaller than '..threshold 2' the sine bulb,\ninbetween both formulas will be interpolated.  Make the second threshold a little smaller than the first for more smoothness.",
        "ASineIntegerPowerBulbWith": "A sine integer power bulb with foldings before, like in the amazing box.",
        "ABulbFormulaAsSuggestedBy": "A bulb formula as suggested by Aexion:\nhttp://www.fractalforums.com/the-3d-mandelbulb/iterating-c/\n\nAdded two options to power-rotate the Constant by the distance of the current vectors of Z and C (Cond Phi=1),\nand the following Mode options:\n(Bits of Mode value)\nBit1: Flip atan theta components\nBit2: Flip atan phi components\nBit3: Flip theta and phi\nBit4: Flip CyCz for angle calc\nBit5: Multiply powerC by the distance of vector Z and C",
        "ChangesTheColouringOnlyNotThe": "changes the colouring only, not the shape",
        "HowTheValueChangesWithThe": "how the value changes with the iteration count; 0 keeps it constant",
        "TheFoldSOffsetOnOne": "the fold's offset on one axis; 1 is the plain shape, other values shift the copies",
        "TheRadiusOfTheSphericalFold": "the radius of the spherical fold's inner sphere; what lies inside is blown up; 0.5 is the classic value, 0 switches it off",
        "RoundsTheEdgesSmallValuesKeep": "rounds the edges; small values keep the shape, large ones melt it",
        "TheWallThicknessOfAHollow": "the wall thickness of a hollow shape",
        "HowManyCopiesWithinCopiesThe": "how many copies within copies the shape has; each level costs time",
        "ASphereInversionOfTheSpace": "a sphere inversion of the space; its centre and radius decide which part is turned inside out",
        "TheJuliaConstantOfThisFormula": "the Julia constant of this formula alone; the scene's Julia mode is in the Formulas window",
        "TheExponentOfTheBulb8": "the exponent of the bulb; 8 gives the classic Mandelbulb, 2 is smooth, higher values give more folds",
        "FineTuningOfTheDistanceEstimate": "fine-tuning of the distance estimate: raise it when the surface shows holes, lower it when it looks swollen",
        "ARotationAppliedEachIterationIn": "a rotation applied each iteration, in degrees",
        "TheBoxFoldSLimitWhat": "the box fold's limit: what lies beyond is mirrored back; 1 is the classic box, smaller folds more",
        "StretchesTheSpaceEachIterationAround": "stretches the space each iteration; around 2 or 3 gives the classic shape, a minus sign mirrors it, values between 1 and 1.5 melt it",
        "ChoosesAVariantOfTheFormula": "chooses a variant of the formula; whole numbers only",
        "TheFourthCoordinateThisStartsIt": "the fourth coordinate: this starts it away from 0; the 4D rotation turns it",
        "ScalesOneCoordinateEachIteration1": "scales one coordinate each iteration; -1 mirrors it, 0 flattens it",
        "TheSizeOfTheBaseShape": "the size of the base shape, in scene units",
        "AConstantAddedEachIterationIt": "a constant added each iteration; it moves the shape and breaks its symmetry",
        "GoodFirstFormulaFewValuesFast": "Good first formula: few values, fast, and example files to start from.",
        "PatienceA4DFormulaWithNumerical": "Patience: a 4D formula with numerical DE renders slowly; start small and raise the detail later.",
        "ATransformationShowsNothingAlonePut": "A transformation shows nothing alone: put a shape in slot 1 and this in slot 2, or the other way round.",
        "WatchThe4DRotationFormulasWindow": "Watch the 4D rotation (Formulas window): it changes the shape completely.",
        "StartFromAnExampleOpenOne": "Start from an example: open one and change the values a little at a time.",
        "ManyValuesChangeOneAtA": "Many values: change one at a time and keep what you like with the History window.",
        "FormulaFile0": "Formula file: {0}",
        "BuiltInFormula": "Built-in formula",
        "Kind0": "Kind: {0}",
        "NoDescriptionFromTheAuthor": "No description from the author.",
        "Values": "Values:",
        "Hints": "Hints:",
        "NoExampleFileUsesItTry": "No example file uses it; try it alone first, then in slot 2 behind an Amazing Box.",
        "ExampleFilesUsingIt0": "Example files using it: {0}",
        "AboutThisFormula": "About this formula...",
    ]

    /// The text of a label as the engine names it (`@MB:Id`), else the
    /// text itself.
    static func text(_ s: String) -> String {
        guard s.hasPrefix("@MB:") else { return s }
        return english[String(s.dropFirst(4))] ?? s
    }

    /// A label in the app's language, or an author's text as it is.
    static func view(_ s: String) -> Text {
        s.hasPrefix("@MB:") ? Text(LocalizedStringKey(text(s))) : Text(verbatim: s)
    }
}
