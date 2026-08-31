package com.android.vending.licensing;

// Callback half of the licensing Binder contract — see
// ILicensingService.aidl for provenance/why-hand-written notes.
oneway interface ILicenseResultListener {
    void verifyLicense(int responseCode, String signedData, String signature);
}
