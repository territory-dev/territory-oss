import Box from '@mui/material/Box'
import List from '@mui/material/List'
import ListItem from '@mui/material/ListItem'
import ListItemButton from '@mui/material/ListItemButton'

import AddIcon from '@mui/icons-material/Add';

import styles from './CreateMapButton.module.css'


export const CreateMapButton = ({ setIsModalOpen }) => {
    return (
        <Box sx={{ width: '100%', maxWidth: 720, marginBottom: '10px', bgcolor: 'background.paper' }}>
            <List>
                <ListItem>
                    <ListItemButton onClick={() => setIsModalOpen(true)} >
                        <AddIcon className={styles.icon} />
                        <div className={styles.text}>New map</div>
                    </ListItemButton>
                </ListItem>
            </List>
        </Box>
    )
}
