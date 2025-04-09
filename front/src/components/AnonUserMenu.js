import { useNavigate } from 'react-router-dom'

import Button from '@mui/material/Button'

export const AnonUserMenu = ({loginRedirect}) => {
    const navigate = useNavigate()

    return <Button
        onClick={() => navigate('/login', { state: { redirect: loginRedirect }})}
    >
        Log In
    </Button>

}
