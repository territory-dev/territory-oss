import React, { useState, useContext } from 'react'

import { useNavigate, useSearchParams } from "react-router-dom"
import Button from '@mui/material/Button'

import { Explained } from '../../components/Explained'
import { SettingsLayout } from '../../components/SettingsLayout'
import { Loader } from '../../components/Loader'
import { maps } from '../../api/api'

export const AuthorizeLocal = () => {
    const [confirmed, setConfirmed] = useState(false)
    const [errorFallback, setErrorFallback] = useState(null)
    const [searchParams, setSearchParams] = useSearchParams()
    const navigate = useNavigate()

    let callbackUrl;
    try{
        callbackUrl = new URL(searchParams.get('callback'))
    } catch (e) {
        callbackUrl = null;
    }

    const handleAuthorize = async () => {
        setConfirmed(true)

        const createTokenResponse = await maps.createUploadToken({
            display_name: searchParams.get('display_name'),
        })

        try {
            const resp = await fetch(
                callbackUrl,
                {method: 'POST', body: JSON.stringify(createTokenResponse)})
        } catch (e) {
            setErrorFallback({
                upload_token: createTokenResponse.upload_token,
            })
            return
        }

        navigate('/upload-tokens')
    }


    if (!callbackUrl) {
        return <div>Invalid URL, callback missing</div>
    }
    console.log(callbackUrl)
    if (callbackUrl.hostname !== 'localhost') {
        return <div>Only localhost callbacks are allowed</div>
    }
    if (callbackUrl.protocol !== 'http:') {
        return <div>Only HTTP allowed for callbacks</div>
    }

    return <SettingsLayout selectedRoute="upload-tokens">
        <h1>Authorize local client</h1>
        <Explained>

            {
                (errorFallback != null)
                ? <div>
                    Unable to talk to Territory CLI.
                    Follow <a href="https://github.com/territory-dev/cli?tab=readme-ov-file#non-interactive-authentication">
                        non-interactive authentication instructions
                    </a> to set an upload token.
                </div>
                : confirmed
                    ?  <Loader />
                    :  <div>
                        <p>
                            A local application asks for permission to create builds and
                            upload code for indexing on your behalf.  Do you consent?
                        </p>
                        <Button onClick={handleAuthorize} variant="contained" color="warning">
                            Authorize
                        </Button>
                    </div>}
        </Explained>
    </SettingsLayout>
}
